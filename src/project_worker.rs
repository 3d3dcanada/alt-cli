//! Blocking project operations stay off terminal and protocol event loops.
//! Cancellation interrupts reads/lock waits; an atomic write already started completes.
use crate::{models::Cancel, project::Project};
use anyhow::{Context, Result, bail, ensure};
use std::{
    path::PathBuf,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

pub async fn run<T, F>(data: PathBuf, cwd: PathBuf, cancel: Cancel, operation: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&mut Project) -> Result<T> + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut project = loop {
            ensure!(!cancel.load(Ordering::Relaxed), "Operation cancelled while waiting; no requested changes applied");
            match Project::open(&data, &cwd) {
                Ok(project) => break project,
                Err(error) if error.chain().any(|e| e.downcast_ref::<std::io::Error>().is_some_and(|e| e.kind() == std::io::ErrorKind::WouldBlock)) => {
                    if Instant::now() >= deadline { bail!("Project is still busy after 10 seconds. Stop its active check or retry when it finishes; your draft is preserved"); }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error),
            }
        };
        project.cancellation = Some(cancel);
        project.check_cancel()?;
        operation(&mut project)
    }).await.context("Project worker stopped unexpectedly")?
}

pub fn task(project: &Project, preferred: Option<String>) -> Result<String> {
    let existing = preferred.or(project.latest_task()?);
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = format!("manual-{}", uuid::Uuid::new_v4());
    project.start_task(&id, "Manual project checks and changes")?;
    Ok(id)
}
