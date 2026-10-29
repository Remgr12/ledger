use erp_kernel::{ErpModule, Migration, ModuleRoute};

pub struct ProjectsModule;

impl Default for ProjectsModule {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectsModule {
    pub fn new() -> Self {
        Self
    }
}

impl ErpModule for ProjectsModule {
    fn id(&self) -> &'static str {
        "mod-projects"
    }

    fn name(&self) -> &'static str {
        "Projects, Tasks & Timesheets"
    }

    fn description(&self) -> &'static str {
        "Project management, task tracking, billable timesheets, and automated invoicing"
    }

    fn dependencies(&self) -> Vec<&'static str> {
        vec!["mod-contacts", "mod-invoicing"]
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![
            Migration {
                version: 1,
                name: "create_projects_tables",
                sql: r#"
                CREATE TABLE IF NOT EXISTS projects (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    code TEXT NOT NULL UNIQUE,
                    customer_id TEXT,
                    status TEXT NOT NULL,
                    billing_rate_hourly TEXT NOT NULL DEFAULT '0',
                    estimated_hours TEXT NOT NULL DEFAULT '0',
                    created_at TEXT NOT NULL,
                    FOREIGN KEY(customer_id) REFERENCES contacts(id)
                );
                CREATE INDEX IF NOT EXISTS idx_projects_code ON projects(code);
                CREATE INDEX IF NOT EXISTS idx_projects_cust ON projects(customer_id);

                CREATE TABLE IF NOT EXISTS project_tasks (
                    id TEXT PRIMARY KEY,
                    project_id TEXT NOT NULL,
                    title TEXT NOT NULL,
                    status TEXT NOT NULL,
                    priority TEXT NOT NULL,
                    estimated_hours TEXT NOT NULL DEFAULT '0',
                    created_at TEXT NOT NULL,
                    FOREIGN KEY(project_id) REFERENCES projects(id)
                );
                CREATE INDEX IF NOT EXISTS idx_ptasks_project ON project_tasks(project_id);

                CREATE TABLE IF NOT EXISTS timesheets (
                    id TEXT PRIMARY KEY,
                    project_id TEXT NOT NULL,
                    task_id TEXT,
                    employee_or_user TEXT NOT NULL,
                    activity_date TEXT NOT NULL,
                    hours TEXT NOT NULL,
                    hourly_rate TEXT NOT NULL,
                    billing_amount TEXT NOT NULL,
                    is_billable INTEGER NOT NULL DEFAULT 1,
                    is_invoiced INTEGER NOT NULL DEFAULT 0,
                    invoice_id TEXT,
                    description TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    FOREIGN KEY(project_id) REFERENCES projects(id),
                    FOREIGN KEY(task_id) REFERENCES project_tasks(id),
                    FOREIGN KEY(invoice_id) REFERENCES invoices(id)
                );
                CREATE INDEX IF NOT EXISTS idx_ts_project ON timesheets(project_id);
                CREATE INDEX IF NOT EXISTS idx_ts_unbilled ON timesheets(project_id, is_billable, is_invoiced);
                "#,
            },
        ]
    }

    fn routes(&self) -> Vec<ModuleRoute> {
        vec![
            ModuleRoute {
                route_id: "projects/list".to_string(),
                title: "Projects & Tasks".to_string(),
                icon: "assignment".to_string(),
                display_order: 70,
            },
            ModuleRoute {
                route_id: "projects/timesheets".to_string(),
                title: "Timesheets".to_string(),
                icon: "schedule".to_string(),
                display_order: 71,
            },
        ]
    }
}
