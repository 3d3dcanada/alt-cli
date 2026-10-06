//! Transactional schema versions with a consistent, recoverable pre-migration copy.
use anyhow::{Result, ensure};
use rusqlite::Connection;
use std::path::Path;

/// Inspect existing state without opening a writer or changing journal/schema.
pub fn compatible(path: &Path, target: i64) -> Result<()> {
    if path.exists() {
        let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        ensure!(
            version <= target,
            "This state was created by a newer Alt version; keep it intact and upgrade Alt"
        );
    }
    Ok(())
}

pub fn migrate(db: &mut Connection, path: &Path, target: i64, sql: &str) -> Result<()> {
    let current: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    ensure!(
        current <= target,
        "This state was created by a newer Alt version; keep it intact and upgrade Alt"
    );
    if current == target {
        return Ok(());
    }
    let tables: i64 = db.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table'",
        [],
        |r| r.get(0),
    )?;
    if tables > 0 {
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("State directory missing"))?
            .join("migrations");
        crate::config::private_dir(&parent)?;
        let backup = parent.join(format!(
            "{}-v{}-{}.sqlite",
            path.file_stem().unwrap_or_default().to_string_lossy(),
            current,
            uuid::Uuid::new_v4()
        ));
        db.execute("VACUUM INTO ?1", [backup.to_string_lossy().as_ref()])?;
    }
    let transaction = db.transaction()?;
    transaction.execute_batch(sql)?;
    transaction.pragma_update(None, "user_version", target)?;
    transaction.commit()?;
    Ok(())
}
