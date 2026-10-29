use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use chrono::Utc;
use parking_lot::RwLock;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::errors::ErpKernelError;
use crate::event_bus::{DomainEvent, EventBus};
use crate::event_log::EventLogStore;
use crate::hlc::HybridLogicalClock;
use crate::module::{ErpModule, ModuleContext, ModuleRoute};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStatus {
    pub id: String,
    pub name: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub is_enabled: bool,
    pub installed_version: u32,
    pub routes: Vec<ModuleRoute>,
}

pub struct ModuleManager {
    db: Database,
    hlc: Arc<HybridLogicalClock>,
    event_bus: Arc<EventBus>,
    registered_modules: RwLock<HashMap<String, Arc<dyn ErpModule>>>,
    enabled_cache: RwLock<HashSet<String>>,
}

impl ModuleManager {
    pub fn new(db: Database, hlc: Arc<HybridLogicalClock>, event_bus: Arc<EventBus>) -> Self {
        Self {
            db,
            hlc,
            event_bus,
            registered_modules: RwLock::new(HashMap::new()),
            enabled_cache: RwLock::new(HashSet::new()),
        }
    }

    /// Connect the event bus to this module manager so all events are routed to enabled modules
    pub fn wire_event_bus(self: &Arc<Self>) {
        let weak_mm = Arc::downgrade(self);
        self.event_bus.subscribe_all(Arc::new(move |event| {
            if let Some(mm) = weak_mm.upgrade() {
                mm.dispatch_event(event)?;
            }
            Ok(())
        }));
    }

    /// Register a module implementation in the registry
    pub fn register(&self, module: Arc<dyn ErpModule>) {
        let mut modules = self.registered_modules.write();
        modules.insert(module.id().to_string(), module);
    }

    /// Initialize the core kernel tables and run migrations for all currently enabled modules
    pub fn initialize(&self) -> Result<(), ErpKernelError> {
        let conn = self.db.conn();

        // 1. Core tables
        EventLogStore::init_schema(&conn)?;

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS system_modules (
                module_id TEXT PRIMARY KEY,
                is_enabled INTEGER NOT NULL DEFAULT 0,
                installed_version INTEGER NOT NULL DEFAULT 0,
                enabled_at TEXT
            );

            CREATE TABLE IF NOT EXISTS schema_migrations (
                module_id TEXT NOT NULL,
                version INTEGER NOT NULL,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL,
                PRIMARY KEY (module_id, version)
            );
            "#,
        )?;

        // 2. Load enabled state from database
        let mut stmt = conn.prepare("SELECT module_id FROM system_modules WHERE is_enabled = 1")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut enabled_set = HashSet::new();
        for r in rows {
            enabled_set.insert(r?);
        }

        // Drop connection lock before acquiring rwlocks and running migrations
        drop(stmt);
        drop(conn);

        *self.enabled_cache.write() = enabled_set.clone();

        // 3. Run migrations for enabled modules
        let sorted_ids = self.resolve_topological_order(&enabled_set)?;
        for id in sorted_ids {
            self.run_module_migrations(&id)?;
        }

        Ok(())
    }

    /// Check if a module is enabled
    pub fn is_enabled(&self, module_id: &str) -> bool {
        self.enabled_cache.read().contains(module_id)
    }

    /// Enable a module, ensuring all its dependencies are met, and run its migrations
    pub fn enable_module(&self, module_id: &str) -> Result<(), ErpKernelError> {
        let module = {
            let mods = self.registered_modules.read();
            mods.get(module_id)
                .cloned()
                .ok_or_else(|| ErpKernelError::ModuleNotFound(module_id.to_string()))?
        };

        // Check dependencies
        let enabled = self.enabled_cache.read();
        for dep in module.dependencies() {
            if !enabled.contains(dep) {
                return Err(ErpKernelError::MissingDependency {
                    module: module_id.to_string(),
                    dependency: dep.to_string(),
                });
            }
        }
        drop(enabled);

        // Run migrations
        self.run_module_migrations(module_id)?;

        // Persist to SQLite
        {
            let conn = self.db.conn();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                r#"
                INSERT INTO system_modules (module_id, is_enabled, enabled_at)
                VALUES (?1, 1, ?2)
                ON CONFLICT(module_id) DO UPDATE SET is_enabled = 1, enabled_at = ?2
                "#,
                params![module_id, now],
            )?;
        }

        self.enabled_cache.write().insert(module_id.to_string());

        // Publish ModuleEnabled event
        let ctx = self.context();
        let _ = ctx.record_and_publish_event(
            "ModuleEnabled",
            "Module",
            module_id,
            serde_json::json!({ "module_id": module_id }),
            "system",
        )?;

        Ok(())
    }

    /// Disable a module, validating that no other enabled modules depend on it
    pub fn disable_module(&self, module_id: &str) -> Result<(), ErpKernelError> {
        let mods = self.registered_modules.read();
        let enabled = self.enabled_cache.read();

        // Check if any active module depends on this one
        for active_id in enabled.iter() {
            if let Some(m) = mods.get(active_id) {
                for dep in m.dependencies() {
                    if dep == module_id {
                        return Err(ErpKernelError::DependentModuleActive {
                            module: module_id.to_string(),
                            dependent: active_id.clone(),
                        });
                    }
                }
            }
        }
        drop(mods);
        drop(enabled);

        {
            let conn = self.db.conn();
            conn.execute(
                "UPDATE system_modules SET is_enabled = 0 WHERE module_id = ?1",
                params![module_id],
            )?;
        }

        self.enabled_cache.write().remove(module_id);

        let ctx = self.context();
        let _ = ctx.record_and_publish_event(
            "ModuleDisabled",
            "Module",
            module_id,
            serde_json::json!({ "module_id": module_id }),
            "system",
        )?;

        Ok(())
    }

    /// Run pending migrations for a specific module
    fn run_module_migrations(&self, module_id: &str) -> Result<(), ErpKernelError> {
        let module = {
            let mods = self.registered_modules.read();
            mods.get(module_id)
                .cloned()
                .ok_or_else(|| ErpKernelError::ModuleNotFound(module_id.to_string()))?
        };

        let migrations = module.migrations();
        if migrations.is_empty() {
            return Ok(());
        }

        let mut conn = self.db.conn();
        let tx = conn.transaction()?;

        // Get already applied versions
        let mut applied_versions = HashSet::new();
        {
            let mut stmt = tx.prepare(
                "SELECT version FROM schema_migrations WHERE module_id = ?1 ORDER BY version ASC",
            )?;
            let rows = stmt.query_map(params![module_id], |row| row.get::<_, u32>(0))?;
            for r in rows {
                applied_versions.insert(r?);
            }
        }

        let mut max_applied_version = 0;
        for m in migrations {
            if !applied_versions.contains(&m.version) {
                tracing::info!(
                    "Executing migration for {}: v{} - {}",
                    module_id,
                    m.version,
                    m.name
                );
                tx.execute_batch(m.sql).map_err(|e| ErpKernelError::MigrationFailed {
                    module: module_id.to_string(),
                    version: m.version,
                    name: m.name.to_string(),
                    error: e.to_string(),
                })?;

                tx.execute(
                    r#"
                    INSERT INTO schema_migrations (module_id, version, name, applied_at)
                    VALUES (?1, ?2, ?3, ?4)
                    "#,
                    params![module_id, m.version, m.name, Utc::now().to_rfc3339()],
                )?;
            }
            if m.version > max_applied_version {
                max_applied_version = m.version;
            }
        }

        tx.execute(
            r#"
            INSERT INTO system_modules (module_id, is_enabled, installed_version)
            VALUES (?1, 1, ?2)
            ON CONFLICT(module_id) DO UPDATE SET installed_version = ?2
            "#,
            params![module_id, max_applied_version],
        )?;

        tx.commit()?;
        Ok(())
    }

    /// Route domain event to all enabled modules
    pub fn dispatch_event(&self, event: &DomainEvent) -> Result<(), ErpKernelError> {
        let enabled = self.enabled_cache.read().clone();
        let mods = self.registered_modules.read().clone();
        let ctx = self.context();

        for mod_id in enabled {
            if let Some(m) = mods.get(&mod_id) {
                m.on_event(event, &ctx)?;
            }
        }

        Ok(())
    }

    /// List all registered modules and their current state
    pub fn list_modules(&self) -> Result<Vec<ModuleStatus>, ErpKernelError> {
        let mods = self.registered_modules.read();
        let enabled = self.enabled_cache.read();
        let conn = self.db.conn();

        let mut installed_versions: HashMap<String, u32> = HashMap::new();
        let mut stmt = conn.prepare("SELECT module_id, installed_version FROM system_modules")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?))
        })?;
        for r in rows {
            let (id, v) = r?;
            installed_versions.insert(id, v);
        }

        let mut list = Vec::new();
        for (id, m) in mods.iter() {
            list.push(ModuleStatus {
                id: id.clone(),
                name: m.name().to_string(),
                description: m.description().to_string(),
                dependencies: m.dependencies().into_iter().map(String::from).collect(),
                is_enabled: enabled.contains(id),
                installed_version: installed_versions.get(id).copied().unwrap_or(0),
                routes: m.routes(),
            });
        }

        // Sort by id for determinism
        list.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(list)
    }

    /// Helper to get ModuleContext
    pub fn context(&self) -> ModuleContext {
        ModuleContext::new(self.db.clone(), self.hlc.clone(), self.event_bus.clone())
    }

    /// Topological sort of module dependencies
    fn resolve_topological_order(&self, module_ids: &HashSet<String>) -> Result<Vec<String>, ErpKernelError> {
        let mods = self.registered_modules.read();
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        fn dfs(
            node: &str,
            mods: &HashMap<String, Arc<dyn ErpModule>>,
            visited: &mut HashSet<String>,
            visiting: &mut HashSet<String>,
            result: &mut Vec<String>,
        ) -> Result<(), ErpKernelError> {
            if visiting.contains(node) {
                return Err(ErpKernelError::Module(format!(
                    "Cyclic dependency detected at module '{}'",
                    node
                )));
            }
            if !visited.contains(node) {
                visiting.insert(node.to_string());
                if let Some(m) = mods.get(node) {
                    for dep in m.dependencies() {
                        dfs(dep, mods, visited, visiting, result)?;
                    }
                }
                visiting.remove(node);
                visited.insert(node.to_string());
                result.push(node.to_string());
            }
            Ok(())
        }

        for id in module_ids {
            if !visited.contains(id) {
                dfs(id, &mods, &mut visited, &mut visiting, &mut result)?;
            }
        }

        Ok(result)
    }
}
