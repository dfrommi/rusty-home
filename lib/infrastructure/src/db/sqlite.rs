use anyhow::{Context as _, Result, ensure};
use rusqlite::{Connection, OpenFlags};
use std::{
    path::Path,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

const BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const SQLITE_READ_CONNECTIONS: usize = 4;

#[derive(Debug)]
struct ReadConnections {
    connections: Vec<Mutex<Connection>>,
    next: AtomicUsize,
}

impl ReadConnections {
    fn next_connection(&self) -> Result<MutexGuard<'_, Connection>> {
        let start = self.next.fetch_add(1, Ordering::Relaxed) % self.connections.len();

        for offset in 0..self.connections.len() {
            let index = (start + offset) % self.connections.len();

            match self.connections[index].try_lock() {
                Ok(connection) => return Ok(connection),
                Err(std::sync::TryLockError::WouldBlock) => continue,
                Err(std::sync::TryLockError::Poisoned(_)) => {
                    return Err(anyhow::anyhow!("SQLite read connection mutex was poisoned"));
                }
            }
        }

        self.connections[start]
            .lock()
            .map_err(|_| anyhow::anyhow!("SQLite read connection mutex was poisoned"))
    }
}

#[derive(Clone)]
pub struct SQLiteDatabase {
    writer: Arc<Mutex<Connection>>,
    readers: Arc<ReadConnections>,
}

impl SQLiteDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        ensure!(!path.as_os_str().is_empty(), "SQLite path must not be empty");

        let flags = OpenFlags::default() - OpenFlags::SQLITE_OPEN_CREATE;
        let connection = Connection::open_with_flags(&path, flags)
            .with_context(|| format!("Error opening existing SQLite database {}", path.display()))?;
        Self::initialize_file(connection, &path)
    }

    pub fn for_writing(&self) -> Result<MutexGuard<'_, Connection>> {
        self.writer
            .lock()
            .map_err(|_| anyhow::anyhow!("SQLite writer connection mutex was poisoned"))
    }

    pub fn for_reading(&self) -> Result<MutexGuard<'_, Connection>> {
        self.readers.next_connection()
    }

    fn initialize_file(writer: Connection, path: &Path) -> Result<Self> {
        writer.busy_timeout(BUSY_TIMEOUT)?;
        writer.pragma_update(None, "journal_mode", "WAL")?;
        writer.pragma_update(None, "synchronous", "FULL")?;
        writer.pragma_update(None, "foreign_keys", true)?;

        let connections = (0..SQLITE_READ_CONNECTIONS)
            .map(|_| {
                let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                    .with_context(|| format!("Error opening SQLite reader {}", path.display()))?;
                connection.busy_timeout(BUSY_TIMEOUT)?;
                connection.pragma_update(None, "foreign_keys", true)?;
                Ok(Mutex::new(connection))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            readers: Arc::new(ReadConnections {
                connections,
                next: AtomicUsize::new(0),
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SQLiteDatabase;

    #[test]
    fn rejects_a_missing_database_without_creating_it() {
        let path = std::env::temp_dir().join(format!("rusty-home-missing-{}.sqlite", std::process::id()));
        assert!(SQLiteDatabase::open(&path).is_err());
        assert!(!path.exists());
    }
}
