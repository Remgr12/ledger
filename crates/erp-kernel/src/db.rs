use std::path::Path;
use std::sync::Arc;
use parking_lot::Mutex;
use rusqlite::Connection;

use crate::errors::ErpKernelError;

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open or create a local SQLite database at the specified path.
    /// In-memory can be specified with ":memory:".
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, ErpKernelError> {
        let conn = Connection::open(path)?;
        Self::configure_connection(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Open an in-memory SQLite database (primarily for testing).
    pub fn open_in_memory() -> Result<Self, ErpKernelError> {
        let conn = Connection::open_in_memory()?;
        Self::configure_connection(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn configure_connection(conn: &Connection) -> Result<(), ErpKernelError> {
        // High-performance local-first SQLite pragmas
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA cache_size = -64000;
            "#,
        )?;
        Ok(())
    }

    /// Access the connection directly with a mutex guard.
    pub fn conn(&self) -> parking_lot::MutexGuard<'_, Connection> {
        self.conn.lock()
    }

    /// Execute a closure within an exclusive SQLite transaction.
    pub fn transaction<T, F>(&self, f: F) -> Result<T, ErpKernelError>
    where
        F: FnOnce(&rusqlite::Transaction<'_>) -> Result<T, ErpKernelError>,
    {
        let mut conn = self.conn.lock();
        let tx = conn.transaction()?;
        let result = f(&tx)?;
        tx.commit()?;
        Ok(result)
    }
}
