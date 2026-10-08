//! Durable drafts are separate from submitted conversation events. A recovered
//! submission is never resent automatically: the user explicitly retries it.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{mpsc, oneshot, watch};

pub const SAVE_INTERVAL: Duration = Duration::from_millis(250);
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Submission {
    pub id: String,
    pub text: String,
}
impl Submission {
    pub fn new(text: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            text,
        }
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Draft {
    pub project: PathBuf,
    pub session: Option<String>,
    pub text: String,
    #[serde(default)]
    pub composer_message_id: Option<String>,
    pub pending: Option<Submission>,
    pub archived: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Book {
    pub active: String,
    pub entries: BTreeMap<String, Draft>,
}
impl Book {
    pub fn load(root: &Path, project: &Path) -> Result<Self> {
        let path = root.join("drafts.json");
        let mut book: Self = if path.exists() {
            ensure!(
                path.metadata()?.len() <= 64 * 1024 * 1024,
                "Saved drafts exceed the 64 MiB reader limit; preserve drafts.json and export older drafts before retrying"
            );
            serde_json::from_slice(&std::fs::read(&path)?).context("Saved drafts cannot be read. Preserve drafts.json before repairing it; no drafts were reset")?
        } else {
            Self::default()
        };
        if !book
            .entries
            .get(&book.active)
            .is_some_and(|d| d.project == project && !d.archived)
        {
            if let Some((id, _)) = book
                .entries
                .iter()
                .find(|(_, d)| d.project == project && !d.archived && d.session.is_none())
            {
                book.active = id.clone();
            } else {
                book.fresh(project);
            }
        }
        Ok(book)
    }
    pub fn fresh(&mut self, project: &Path) {
        self.active = uuid::Uuid::new_v4().to_string();
        self.entries.insert(
            self.active.clone(),
            Draft {
                project: project.into(),
                ..Default::default()
            },
        );
    }
    pub fn current(&self) -> &Draft {
        self.entries.get(&self.active).expect("Active draft")
    }
    pub fn current_mut(&mut self) -> &mut Draft {
        self.entries.get_mut(&self.active).expect("Active draft")
    }
    pub fn save(&self, root: &Path) -> Result<()> {
        let bytes = serde_json::to_vec(self)?;
        ensure!(
            bytes.len() <= 64 * 1024 * 1024,
            "Draft storage is full. Export or explicitly discard retained drafts; current text is still in the editor"
        );
        crate::config::atomic_write(&root.join("drafts.json"), &bytes)
    }
    pub fn remember(
        &mut self,
        project: &Path,
        session: Option<&str>,
        text: &str,
        pending: Option<&Submission>,
        composer_message_id: Option<&str>,
    ) {
        let d = self.current_mut();
        d.project = project.into();
        d.session = session.map(str::to_owned);
        d.text = text.into();
        d.pending = pending.cloned();
        d.composer_message_id = composer_message_id.map(str::to_owned);
    }
    pub fn activate(&mut self, project: &Path, session: Option<&str>) {
        if let Some((id, _)) = self
            .entries
            .iter()
            .find(|(_, d)| d.project == project && d.session.as_deref() == session && !d.archived)
        {
            self.active = id.clone();
        } else {
            self.fresh(project);
            self.current_mut().session = session.map(str::to_owned);
        }
    }
}

/// One TUI owns a draft book. The worker retains this lease until its last
/// write finishes, including when the terminal loop exits unexpectedly.
pub fn lease(root: &Path) -> Result<Arc<std::fs::File>> {
    let path = root.join("drafts.lock");
    ensure!(
        !path.is_symlink(),
        "Draft writer lock cannot be a symbolic link"
    );
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    fs2::FileExt::try_lock_exclusive(&file).context("Another Alt window owns drafts in this state folder. Return to that window or close it before reopening here. Use a different --data-dir for an independent workspace; existing drafts were not changed")?;
    Ok(Arc::new(file))
}
#[derive(Clone)]
struct Snapshot {
    revision: u64,
    book: Book,
}
type FlushReply = oneshot::Sender<Result<()>>;
pub struct Writer {
    latest: watch::Sender<Snapshot>,
    flush: mpsc::Sender<FlushReply>,
    revision: u64,
    pub errors: mpsc::Receiver<String>,
    pub saved: watch::Receiver<Option<u64>>,
}
impl Writer {
    pub fn start(root: PathBuf, lease: Arc<std::fs::File>) -> Self {
        let (latest, mut updates) = watch::channel(Snapshot {
            revision: 0,
            book: Book::default(),
        });
        let (flush, mut requests) = mpsc::channel::<FlushReply>(1);
        let (errors_tx, errors) = mpsc::channel(4);
        let (saved_tx, saved) = watch::channel(None);
        tokio::spawn(async move {
            let _lease = lease;
            let mut pending = false;
            let mut interval = tokio::time::interval(SAVE_INTERVAL);
            loop {
                let reply = tokio::select! {
                    changed=updates.changed()=>{ if changed.is_err(){break;} pending=true;continue; },
                    reply=requests.recv()=>{let Some(reply)=reply else {break;};Some(reply)},
                    _=interval.tick(),if pending=>None,
                };
                let snapshot = updates.borrow_and_update().clone();
                pending = false;
                let root = root.clone();
                let revision = snapshot.revision;
                let result = tokio::task::spawn_blocking(move || snapshot.book.save(&root))
                    .await
                    .context("Draft writer stopped")
                    .and_then(|r| r);
                if result.is_ok() {
                    let _ = saved_tx.send(Some(revision));
                }
                if let Some(reply) = reply {
                    let _ = reply.send(result);
                } else if let Err(error) = result {
                    let _=errors_tx.try_send(format!("Draft not saved: {error:#}. Your text remains here. Free storage or repair the state folder, then use Save drafts before leaving."));
                }
            }
        });
        Self {
            latest,
            flush,
            revision: 0,
            errors,
            saved,
        }
    }
    pub fn queue(&mut self, book: Book) -> Result<()> {
        self.revision = self
            .revision
            .checked_add(1)
            .context("Draft revision counter exhausted; save and restart")?;
        self.latest
            .send(Snapshot {
                revision: self.revision,
                book,
            })
            .context("Draft writer stopped; current text remains in the editor")
    }
    pub fn current_revision(&self) -> u64 {
        self.revision
    }
    pub async fn flush(&mut self, book: Book) -> Result<()> {
        self.queue(book)?;
        let (reply, result) = oneshot::channel();
        self.flush
            .send(reply)
            .await
            .context("Draft writer stopped")?;
        result.await.context("Draft save did not complete")?
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn saved_drafts_keep_pending_ids_archives_and_unicode_across_restart() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project");
        let mut book = Book::load(dir.path(), &project).unwrap();
        let pending = Submission::new("submitted request".into());
        book.remember(
            &project,
            None,
            "next requirement 日本語",
            Some(&pending),
            None,
        );
        let first = book.active.clone();
        book.current_mut().archived = true;
        book.fresh(&project);
        let mut writer = Writer::start(dir.path().into(), lease(dir.path()).unwrap());
        writer.flush(book.clone()).await.unwrap();
        let loaded = Book::load(dir.path(), &project).unwrap();
        assert_eq!(loaded, book);
        assert_eq!(
            loaded.entries[&first].pending.as_ref().unwrap().id,
            pending.id
        );
        assert_eq!(loaded.entries[&first].text, "next requirement 日本語");
    }
    #[tokio::test]
    async fn failed_flush_is_reported_and_does_not_claim_durability() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("drafts.json")).unwrap();
        let mut writer = Writer::start(dir.path().into(), lease(dir.path()).unwrap());
        let mut book = Book::default();
        book.fresh(dir.path());
        book.current_mut().text = "keep me".into();
        assert!(writer.flush(book.clone()).await.is_err());
        assert_eq!(book.current().text, "keep me");
    }
    #[tokio::test]
    async fn second_window_cannot_overwrite_drafts_and_latest_revision_is_coalesced() {
        let dir = tempfile::tempdir().unwrap();
        let lock = lease(dir.path()).unwrap();
        let mut writer = Writer::start(dir.path().into(), lock.clone());
        let mut book = Book::load(dir.path(), dir.path()).unwrap();
        assert!(lease(dir.path()).is_err());
        for n in 0..200 {
            book.current_mut().text = format!("latest {n}");
            writer.queue(book.clone()).unwrap();
        }
        writer.flush(book).await.unwrap();
        assert_eq!(
            Book::load(dir.path(), dir.path()).unwrap().current().text,
            "latest 199"
        );
        drop(lock);
        assert!(
            lease(dir.path()).is_err(),
            "Worker keeps its lease until the last write finishes"
        );
        drop(writer);
        for _ in 0..20 {
            if lease(dir.path()).is_ok() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("Draft worker failed to release its lease");
    }
}
