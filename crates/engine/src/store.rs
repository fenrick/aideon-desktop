//! Persistence abstractions for the commit model.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use rusqlite::{Connection, OptionalExtension, params};
use tokio::task::spawn_blocking;

use crate::error::{PraxisError, PraxisResult};
use crate::temporal::{CommitSummary, PersistedCommit};

#[async_trait]
pub trait Store: Send + Sync {
    async fn list_branches(&self) -> PraxisResult<Vec<(String, Option<String>)>>;
    async fn ensure_branch(&self, name: &str) -> PraxisResult<()>;
    async fn get_branch_head(&self, name: &str) -> PraxisResult<Option<String>>;
    async fn compare_and_swap_branch(
        &self,
        name: &str,
        expected: Option<&str>,
        next: Option<&str>,
    ) -> PraxisResult<()>;
    async fn put_commit(&self, commit: &PersistedCommit) -> PraxisResult<()>;
    async fn get_commit(&self, id: &str) -> PraxisResult<Option<PersistedCommit>>;
    async fn put_tag(&self, tag: &str, commit_id: &str) -> PraxisResult<()>;
    async fn get_tag(&self, tag: &str) -> PraxisResult<Option<String>>;
}

#[derive(Clone, Default)]
pub struct MemoryStore {
    branches: Arc<Mutex<HashMap<String, Option<String>>>>,
    commits: Arc<Mutex<HashMap<String, PersistedCommit>>>,
    tags: Arc<Mutex<HashMap<String, String>>>,
}

#[async_trait]
impl Store for MemoryStore {
    async fn list_branches(&self) -> PraxisResult<Vec<(String, Option<String>)>> {
        let guard = self
            .branches
            .lock()
            .map_err(|_| PraxisError::IntegrityViolation {
                message: "branch store poisoned".into(),
            })?;
        Ok(guard
            .iter()
            .map(|(name, head)| (name.clone(), head.clone()))
            .collect())
    }

    async fn ensure_branch(&self, name: &str) -> PraxisResult<()> {
        let mut guard = self
            .branches
            .lock()
            .map_err(|_| PraxisError::IntegrityViolation {
                message: "branch store poisoned".into(),
            })?;
        guard.entry(name.to_string()).or_insert(None);
        Ok(())
    }

    async fn get_branch_head(&self, name: &str) -> PraxisResult<Option<String>> {
        let guard = self
            .branches
            .lock()
            .map_err(|_| PraxisError::IntegrityViolation {
                message: "branch store poisoned".into(),
            })?;
        Ok(guard.get(name).cloned().flatten())
    }

    async fn compare_and_swap_branch(
        &self,
        name: &str,
        expected: Option<&str>,
        next: Option<&str>,
    ) -> PraxisResult<()> {
        let mut guard = self
            .branches
            .lock()
            .map_err(|_| PraxisError::IntegrityViolation {
                message: "branch store poisoned".into(),
            })?;
        let current = guard.get(name).cloned().flatten();
        if current.as_deref() != expected {
            return Err(PraxisError::ConcurrencyConflict {
                branch: name.to_string(),
                expected: expected.map(|v| v.to_string()),
                actual: current,
            });
        }
        guard.insert(name.to_string(), next.map(|v| v.to_string()));
        Ok(())
    }

    async fn put_commit(&self, commit: &PersistedCommit) -> PraxisResult<()> {
        let mut guard = self
            .commits
            .lock()
            .map_err(|_| PraxisError::IntegrityViolation {
                message: "commit store poisoned".into(),
            })?;
        guard.insert(commit.summary.id.clone(), commit.clone());
        Ok(())
    }

    async fn get_commit(&self, id: &str) -> PraxisResult<Option<PersistedCommit>> {
        let guard = self
            .commits
            .lock()
            .map_err(|_| PraxisError::IntegrityViolation {
                message: "commit store poisoned".into(),
            })?;
        Ok(guard.get(id).cloned())
    }

    async fn put_tag(&self, tag: &str, commit_id: &str) -> PraxisResult<()> {
        let mut guard = self
            .tags
            .lock()
            .map_err(|_| PraxisError::IntegrityViolation {
                message: "tag store poisoned".into(),
            })?;
        guard.insert(tag.to_string(), commit_id.to_string());
        Ok(())
    }

    async fn get_tag(&self, tag: &str) -> PraxisResult<Option<String>> {
        let guard = self
            .tags
            .lock()
            .map_err(|_| PraxisError::IntegrityViolation {
                message: "tag store poisoned".into(),
            })?;
        Ok(guard.get(tag).cloned())
    }
}

#[derive(Clone)]
pub struct SqliteDb {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteDb {
    pub async fn open(path: impl AsRef<Path>) -> PraxisResult<Self> {
        let path = path.as_ref().to_owned();
        let conn = spawn_blocking(move || Connection::open(path))
            .await
            .map_err(|err| PraxisError::IntegrityViolation {
                message: format!("sqlite join error: {err}"),
            })
            .and_then(|result| {
                result.map_err(|err| PraxisError::IntegrityViolation {
                    message: format!("sqlite open error: {err}"),
                })
            })?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema().await?;
        Ok(db)
    }

    async fn init_schema(&self) -> PraxisResult<()> {
        self.with_conn(|conn| {
            conn.execute_batch(
                "BEGIN;
                 CREATE TABLE IF NOT EXISTS branches (
                     name TEXT PRIMARY KEY,
                     head TEXT
                 );
                 CREATE TABLE IF NOT EXISTS commits (
                     id TEXT PRIMARY KEY,
                     summary_json TEXT NOT NULL,
                     change_set_json TEXT NOT NULL
                 );
                 CREATE TABLE IF NOT EXISTS tags (
                     tag TEXT PRIMARY KEY,
                     commit_id TEXT NOT NULL
                 );
                 COMMIT;",
            )
        })
        .await
    }

    async fn with_conn<F, T>(&self, func: F) -> PraxisResult<T>
    where
        F: FnOnce(&Connection) -> Result<T, rusqlite::Error> + Send + 'static,
        T: Send + 'static,
    {
        let conn = Arc::clone(&self.conn);
        let result = spawn_blocking(move || {
            let guard = conn.lock().map_err(|_| rusqlite::Error::InvalidQuery)?;
            func(&guard)
        })
        .await
        .map_err(|err| PraxisError::IntegrityViolation {
            message: format!("sqlite join error: {err}"),
        })?;
        result.map_err(|err| PraxisError::IntegrityViolation {
            message: format!("sqlite error: {err}"),
        })
    }

    fn commit_from_row(
        summary_json: String,
        change_set_json: String,
    ) -> PraxisResult<PersistedCommit> {
        let summary: CommitSummary =
            serde_json::from_str(&summary_json).map_err(|err| PraxisError::IntegrityViolation {
                message: format!("commit summary decode error: {err}"),
            })?;
        let change_set = serde_json::from_str(&change_set_json).map_err(|err| {
            PraxisError::IntegrityViolation {
                message: format!("commit change set decode error: {err}"),
            }
        })?;
        Ok(PersistedCommit {
            summary,
            change_set,
        })
    }
}

#[async_trait]
impl Store for SqliteDb {
    async fn list_branches(&self) -> PraxisResult<Vec<(String, Option<String>)>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT name, head FROM branches")?;
            let rows = stmt
                .query_map([], |row| {
                    let name: String = row.get(0)?;
                    let head: Option<String> = row.get(1)?;
                    Ok((name, head))
                })?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(rows)
        })
        .await
    }

    async fn ensure_branch(&self, name: &str) -> PraxisResult<()> {
        let name = name.to_string();
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO branches (name, head) VALUES (?1, NULL)",
                params![name],
            )?;
            Ok(())
        })
        .await
    }

    async fn get_branch_head(&self, name: &str) -> PraxisResult<Option<String>> {
        let name = name.to_string();
        self.with_conn(move |conn| {
            conn.query_row(
                "SELECT head FROM branches WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .optional()
        })
        .await
    }

    async fn compare_and_swap_branch(
        &self,
        name: &str,
        expected: Option<&str>,
        next: Option<&str>,
    ) -> PraxisResult<()> {
        let name = name.to_string();
        let expected = expected.map(|v| v.to_string());
        let next = next.map(|v| v.to_string());
        let conn = Arc::clone(&self.conn);
        let result = spawn_blocking(move || -> PraxisResult<()> {
            let guard = conn.lock().map_err(|_| PraxisError::IntegrityViolation {
                message: "sqlite connection poisoned".into(),
            })?;
            let tx = guard
                .transaction()
                .map_err(|err| PraxisError::IntegrityViolation {
                    message: format!("sqlite transaction error: {err}"),
                })?;
            tx.execute(
                "INSERT OR IGNORE INTO branches (name, head) VALUES (?1, NULL)",
                params![name],
            )
            .map_err(|err| PraxisError::IntegrityViolation {
                message: format!("sqlite insert branch error: {err}"),
            })?;
            let current: Option<String> = tx
                .query_row(
                    "SELECT head FROM branches WHERE name = ?1",
                    params![name],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|err| PraxisError::IntegrityViolation {
                    message: format!("sqlite read branch error: {err}"),
                })?;
            if current != expected {
                return Err(PraxisError::ConcurrencyConflict {
                    branch: name,
                    expected,
                    actual: current,
                });
            }
            tx.execute(
                "UPDATE branches SET head = ?1 WHERE name = ?2",
                params![next, name],
            )
            .map_err(|err| PraxisError::IntegrityViolation {
                message: format!("sqlite update branch error: {err}"),
            })?;
            tx.commit().map_err(|err| PraxisError::IntegrityViolation {
                message: format!("sqlite commit error: {err}"),
            })?;
            Ok(())
        })
        .await
        .map_err(|err| PraxisError::IntegrityViolation {
            message: format!("sqlite join error: {err}"),
        })?;
        result
    }

    async fn put_commit(&self, commit: &PersistedCommit) -> PraxisResult<()> {
        let summary_json = serde_json::to_string(&commit.summary).map_err(|err| {
            PraxisError::IntegrityViolation {
                message: format!("commit summary encode error: {err}"),
            }
        })?;
        let change_set_json = serde_json::to_string(&commit.change_set).map_err(|err| {
            PraxisError::IntegrityViolation {
                message: format!("commit change set encode error: {err}"),
            }
        })?;
        let id = commit.summary.id.clone();
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT INTO commits (id, summary_json, change_set_json) VALUES (?1, ?2, ?3)",
                params![id, summary_json, change_set_json],
            )?;
            Ok(())
        })
        .await
    }

    async fn get_commit(&self, id: &str) -> PraxisResult<Option<PersistedCommit>> {
        let id = id.to_string();
        self.with_conn(move |conn| {
            conn.query_row(
                "SELECT summary_json, change_set_json FROM commits WHERE id = ?1",
                params![id],
                |row| {
                    let summary_json: String = row.get(0)?;
                    let change_set_json: String = row.get(1)?;
                    Ok((summary_json, change_set_json))
                },
            )
            .optional()
        })
        .await
        .and_then(|result| match result {
            Some((summary_json, change_set_json)) => {
                Self::commit_from_row(summary_json, change_set_json).map(Some)
            }
            None => Ok(None),
        })
    }

    async fn put_tag(&self, tag: &str, commit_id: &str) -> PraxisResult<()> {
        let tag = tag.to_string();
        let commit_id = commit_id.to_string();
        self.with_conn(move |conn| {
            conn.execute(
                "INSERT OR REPLACE INTO tags (tag, commit_id) VALUES (?1, ?2)",
                params![tag, commit_id],
            )?;
            Ok(())
        })
        .await
    }

    async fn get_tag(&self, tag: &str) -> PraxisResult<Option<String>> {
        let tag = tag.to_string();
        self.with_conn(move |conn| {
            conn.query_row(
                "SELECT commit_id FROM tags WHERE tag = ?1",
                params![tag],
                |row| row.get(0),
            )
            .optional()
        })
        .await
    }
}
