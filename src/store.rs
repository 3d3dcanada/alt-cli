use crate::config::Profile;
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub struct Store(Connection);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub engine_id: String,
    pub cwd: String,
    pub profile_name: String,
    pub profile: Profile,
}

#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub cwd: String,
    pub model: String,
    pub updated: String,
    pub archived: bool,
}

impl Store {
    pub fn open(root: &Path) -> Result<Self> {
        crate::config::private_dir(root)?;
        crate::schema::compatible(&root.join("sessions.db"), 1)?;
        let mut db = Connection::open(root.join("sessions.db"))?;
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("PRAGMA journal_mode=WAL;")?;
        crate::schema::migrate(&mut db,&root.join("sessions.db"),1,concat!("
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY, engine_id TEXT NOT NULL, cwd TEXT NOT NULL,
                profile_name TEXT NOT NULL, profile TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
            CREATE TABLE IF NOT EXISTS events (
                seq INTEGER PRIMARY KEY, session_id TEXT NOT NULL,
                payload TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
            CREATE INDEX IF NOT EXISTS events_by_session ON events(session_id, seq);", "CREATE TABLE IF NOT EXISTS session_details (
            session_id TEXT PRIMARY KEY, title TEXT NOT NULL DEFAULT '', archived INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);
            CREATE TABLE IF NOT EXISTS project_briefs (cwd TEXT PRIMARY KEY, body TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP);"))?;
        Ok(Self(db))
    }

    pub fn create(&self, session: &Session) -> Result<()> {
        let tx = self.0.unchecked_transaction()?;
        tx.execute("INSERT INTO sessions(id, engine_id, cwd, profile_name, profile) VALUES (?1,?2,?3,?4,?5)",
            params![session.id, session.engine_id, session.cwd, session.profile_name, serde_json::to_string(&session.profile)?])?;
        tx.execute(
            "INSERT INTO session_details(session_id) VALUES (?1)",
            [&session.id],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Session> {
        let (id, engine_id, cwd, profile_name, profile): (String, String, String, String, String) =
            self.0
                .query_row(
                    "SELECT id, engine_id, cwd, profile_name, profile FROM sessions WHERE id=?1",
                    [id],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    },
                )
                .context("Saved session not found")?;
        Ok(Session {
            id,
            engine_id,
            cwd,
            profile_name,
            profile: serde_json::from_str(&profile)?,
        })
    }

    pub fn list(&self) -> Result<Vec<(String, String, String)>> {
        let mut stmt = self.0.prepare(
            "SELECT id, profile_name, cwd FROM sessions ORDER BY created_at DESC, rowid DESC",
        )?;
        Ok(stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?)
    }

    /// Explicitly updates only allocation on an existing model selection. Keep
    /// the durable session and the audit event in the same transaction.
    pub fn update_allocation(&self, id: &str, profile: Profile) -> Result<Session> {
        profile.validate()?;
        let mut session = self.get(id)?;
        let before = session.profile.clone();
        let mut identity = profile.clone();
        identity.context_tokens = before.context_tokens;
        identity.max_turns = before.max_turns;
        identity.inference = before.inference.clone();
        ensure!(
            serde_json::to_value(&identity)? == serde_json::to_value(&before)?,
            "Allocation recovery cannot change the saved model, provider, endpoint or authentication binding"
        );
        let tx = self.0.unchecked_transaction()?;
        tx.execute(
            "UPDATE sessions SET profile=?1 WHERE id=?2",
            params![serde_json::to_string(&profile)?, id],
        )?;
        let event = serde_json::json!({"type":"allocation_changed","before":{"context_tokens":before.context_tokens,"max_turns":before.max_turns,"inference":before.inference},"after":{"context_tokens":profile.context_tokens,"max_turns":profile.max_turns,"inference":profile.inference},"model_unchanged":true,"scope":"Explicit saved allocation update; next connection records a new allowance"});
        tx.execute(
            "INSERT INTO events(session_id,payload) VALUES(?1,?2)",
            params![id, serde_json::to_string(&event)?],
        )?;
        tx.commit()?;
        session.profile = profile;
        Ok(session)
    }

    pub fn append(&self, id: &str, event: &serde_json::Value) -> Result<()> {
        let tx = self.0.unchecked_transaction()?;
        ensure!(
            tx.query_row("SELECT count(*) FROM sessions WHERE id=?1", [id], |r| r
                .get::<_, i64>(0))?
                == 1,
            "Cannot append to an unknown session"
        );
        tx.execute(
            "INSERT INTO events(session_id, payload) VALUES (?1,?2)",
            params![id, serde_json::to_string(event)?],
        )?;
        if event["type"] == "user" {
            let title: String = event["text"]
                .as_str()
                .unwrap_or("New conversation")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(80)
                .collect();
            tx.execute("INSERT INTO session_details(session_id,title) VALUES (?1,?2)
                ON CONFLICT(session_id) DO UPDATE SET title=CASE WHEN title='' THEN excluded.title ELSE title END, updated_at=CURRENT_TIMESTAMP", params![id,title])?;
        } else if event["type"] == "turn_end" || event["type"] == "error" {
            tx.execute(
                "UPDATE session_details SET updated_at=CURRENT_TIMESTAMP WHERE session_id=?1",
                [id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn summaries(&self, filter: &str, archived: bool) -> Result<Vec<SessionSummary>> {
        let mut stmt = self.0.prepare("SELECT s.id, COALESCE(NULLIF(d.title,''),'New conversation'), s.cwd,
            COALESCE(json_extract(s.profile,'$.model'),s.profile_name), COALESCE(d.updated_at,s.created_at), COALESCE(d.archived,0)
            FROM sessions s LEFT JOIN session_details d ON d.session_id=s.id
            WHERE COALESCE(d.archived,0)=?1 AND (?2='' OR instr(lower(COALESCE(d.title,'') || ' ' || s.cwd || ' ' || s.profile),lower(?2)) > 0)
            ORDER BY COALESCE(d.updated_at,s.created_at) DESC,s.rowid DESC LIMIT 500")?;
        Ok(stmt
            .query_map(params![archived, filter], |r| {
                Ok(SessionSummary {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    cwd: r.get(2)?,
                    model: r.get(3)?,
                    updated: r.get(4)?,
                    archived: r.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?)
    }

    pub fn rename(&self, id: &str, title: &str) -> Result<()> {
        self.get(id)?;
        ensure!(
            !title.trim().is_empty() && title.chars().count() <= 120,
            "Use a title between 1 and 120 characters"
        );
        self.0.execute(
            "INSERT INTO session_details(session_id,title) VALUES (?1,?2)
            ON CONFLICT(session_id) DO UPDATE SET title=excluded.title",
            params![id, title.trim()],
        )?;
        Ok(())
    }

    pub fn archive(&self, id: &str, archived: bool) -> Result<()> {
        self.get(id)?;
        self.0.execute(
            "INSERT INTO session_details(session_id,archived) VALUES (?1,?2)
            ON CONFLICT(session_id) DO UPDATE SET archived=excluded.archived",
            params![id, archived],
        )?;
        Ok(())
    }

    pub fn brief(&self, cwd: &Path) -> Result<String> {
        use rusqlite::OptionalExtension;
        Ok(self
            .0
            .query_row(
                "SELECT body FROM project_briefs WHERE cwd=?1",
                [cwd.to_string_lossy().as_ref()],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or_default())
    }

    pub fn save_brief(&self, cwd: &Path, body: &str) -> Result<()> {
        ensure!(
            body.len() <= 16_000,
            "Keep the project brief under 16 KB so it leaves room for your conversation"
        );
        self.0.execute(
            "INSERT INTO project_briefs(cwd,body) VALUES (?1,?2)
            ON CONFLICT(cwd) DO UPDATE SET body=excluded.body,updated_at=CURRENT_TIMESTAMP",
            params![cwd.to_string_lossy(), body],
        )?;
        Ok(())
    }

    pub fn export_files(&self, root: &Path, id: &str) -> Result<std::path::PathBuf> {
        use std::io::Write;
        let session = self.get(id)?;
        let directory =
            root.join("exports")
                .join(format!("{}-{}", id, uuid::Uuid::new_v4().simple()));
        crate::config::private_dir(&directory)?;
        let mut raw = std::fs::File::create(directory.join("events.jsonl"))?;
        let mut report = std::fs::File::create(directory.join("conversation.md"))?;
        writeln!(
            raw,
            "{}",
            serde_json::json!({"type":"session","session":session})
        )?;
        writeln!(
            report,
            "# Alt conversation\n\nProject: {}\n\nModel: {}\n",
            session.cwd, session.profile.model
        )?;
        self.visit_history(id, |event| {
            writeln!(raw, "{event}")?;
            match event["type"].as_str() {
                Some("user") => writeln!(
                    report,
                    "\n\n## You\n\n{}\n\n## Assistant\n",
                    event["text"].as_str().unwrap_or("")
                )?,
                Some("update") => {
                    if let Some(text) = crate::engine::update_text(&event["data"]) {
                        write!(report, "{text}")?;
                    }
                }
                Some("error") => {
                    writeln!(report, "\nError: {}", event["text"].as_str().unwrap_or(""))?
                }
                _ => {}
            }
            Ok(())
        })?;
        raw.sync_all()?;
        report.sync_all()?;
        Ok(directory)
    }

    pub fn history(&self, id: &str) -> Result<Vec<serde_json::Value>> {
        let mut events = Vec::new();
        self.visit_history(id, |event| {
            events.push(event);
            Ok(())
        })?;
        Ok(events)
    }

    /// A bounded tail for interactive recovery. Complete evidence stays in SQLite
    /// and remains available through the streaming export.
    pub fn recent_history(&self, id: &str) -> Result<Vec<serde_json::Value>> {
        self.get(id)?;
        let mut stmt = self.0.prepare(
            "SELECT payload FROM events WHERE session_id=?1 ORDER BY seq DESC LIMIT 600",
        )?;
        let mut events = Vec::new();
        let mut bytes = 0;
        for row in stmt.query_map([id], |row| row.get::<_, String>(0))? {
            let row = row?;
            bytes += row.len();
            if bytes > 4 * 1024 * 1024 {
                break;
            }
            events.push(serde_json::from_str(&row)?);
        }
        events.reverse();
        Ok(events)
    }

    /// Stream rows so displaying/exporting long sessions need not load the full
    /// evidence database into memory.
    pub fn visit_history(
        &self,
        id: &str,
        mut visit: impl FnMut(serde_json::Value) -> Result<()>,
    ) -> Result<()> {
        self.get(id)?;
        let mut stmt = self
            .0
            .prepare("SELECT payload FROM events WHERE session_id=?1 ORDER BY seq")?;
        let rows = stmt.query_map([id], |row| row.get::<_, String>(0))?;
        for row in rows {
            visit(serde_json::from_str(&row?)?)?;
        }
        Ok(())
    }
}

const HISTORY_LINE_LIMIT: usize = 16 * 1024 * 1024;

impl Store {
    /// User-selected archived conversations only; write a compressed recovery
    /// archive before deleting any rows. A transaction protects concurrent updates.
    pub fn retain_archived(
        &mut self,
        root: &Path,
        days: u64,
        apply: bool,
    ) -> Result<serde_json::Value> {
        use std::io::Write;
        ensure!(days >= 1, "Keep at least one day of conversation history");
        let tx = self
            .0
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let rows:Vec<(String,String)>=tx.prepare("SELECT s.id,d.title FROM sessions s JOIN session_details d ON d.session_id=s.id WHERE d.archived=1 AND d.updated_at < datetime('now',?1) ORDER BY d.updated_at")?.query_map([format!("-{days} days")],|r|Ok((r.get(0)?,r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        let mut archived = Vec::new();
        for (id, title) in &rows {
            if !apply {
                continue;
            }
            let session: Session = tx.query_row(
                "SELECT id,engine_id,cwd,profile_name,profile FROM sessions WHERE id=?1",
                [id],
                |r| {
                    let raw: String = r.get(4)?;
                    Ok(Session {
                        id: r.get(0)?,
                        engine_id: r.get(1)?,
                        cwd: r.get(2)?,
                        profile_name: r.get(3)?,
                        profile: serde_json::from_str(&raw).map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                4,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?,
                    })
                },
            )?;
            let folder = root.join("history-archives");
            crate::config::private_dir(&folder)?;
            let temp = tempfile::NamedTempFile::new_in(&folder)?;
            let mut output = flate2::write::GzEncoder::new(temp, flate2::Compression::default());
            let header =
                serde_json::json!({"format":1,"session":session,"title":title}).to_string();
            ensure!(
                header.len() < HISTORY_LINE_LIMIT,
                "Conversation metadata exceeds the restorable archive limit; original retained"
            );
            writeln!(output, "{header}")?;
            let mut statement =
                tx.prepare("SELECT payload FROM events WHERE session_id=?1 ORDER BY seq")?;
            let mut bytes = header.len() + 1;
            for row in statement.query_map([id], |r| r.get::<_, String>(0))? {
                let line = row?;
                ensure!(
                    line.len() < HISTORY_LINE_LIMIT,
                    "Conversation event exceeds the restorable archive limit; original retained"
                );
                bytes += line.len() + 1;
                ensure!(
                    bytes <= 512 * 1024 * 1024,
                    "Conversation exceeds archive limit; export it explicitly before retention"
                );
                writeln!(output, "{line}")?;
            }
            let temp = output.finish()?;
            temp.as_file().sync_all()?;
            let path = folder.join(format!(
                "{}-{}.jsonl.gz",
                crate::project::digest(id.as_bytes()),
                uuid::Uuid::new_v4()
            ));
            temp.persist_noclobber(&path).map_err(|e| e.error)?;
            std::fs::File::open(&folder)?.sync_all()?;
            archived.push(path);
        }
        if apply {
            for (id, _) in &rows {
                tx.execute("DELETE FROM events WHERE session_id=?1", [id])?;
                tx.execute("DELETE FROM session_details WHERE session_id=?1", [id])?;
                tx.execute("DELETE FROM sessions WHERE id=?1", [id])?;
            }
        }
        tx.commit()?;
        let compacted = apply
            && self
                .0
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")
                .is_ok();
        Ok(
            serde_json::json!({"applied":apply,"keep_days":days,"conversations":rows,"recovery_archives":archived,"database_compacted":compacted,"preserved":"Unarchived and recent conversations, project briefs/checkpoints and model files"}),
        )
    }
    pub fn restore_history(&mut self, path: &Path) -> Result<String> {
        use std::io::{BufRead, Read};
        let mut reader =
            std::io::BufReader::new(flate2::read::GzDecoder::new(std::fs::File::open(path)?));
        let read_line = |reader: &mut std::io::BufReader<
            flate2::read::GzDecoder<std::fs::File>,
        >|
         -> Result<String> {
            let mut line = String::new();
            reader
                .take(HISTORY_LINE_LIMIT as u64 + 1)
                .read_line(&mut line)?;
            ensure!(
                line.len() <= HISTORY_LINE_LIMIT,
                "Conversation archive line too large"
            );
            Ok(line)
        };
        let first_line = read_line(&mut reader)?;
        let mut bytes = first_line.len();
        let header: serde_json::Value = serde_json::from_str(&first_line)?;
        ensure!(header["format"] == 1, "Unsupported conversation archive");
        let session: Session = serde_json::from_value(header["session"].clone())?;
        let tx = self.0.transaction()?;
        let exists: u64 = tx.query_row(
            "SELECT count(*) FROM sessions WHERE id=?1",
            [&session.id],
            |r| r.get(0),
        )?;
        ensure!(
            exists == 0,
            "Conversation already exists; restore never overwrites history"
        );
        tx.execute(
            "INSERT INTO sessions(id,engine_id,cwd,profile_name,profile) VALUES(?1,?2,?3,?4,?5)",
            params![
                session.id,
                session.engine_id,
                session.cwd,
                session.profile_name,
                serde_json::to_string(&session.profile)?
            ],
        )?;
        tx.execute(
            "INSERT INTO session_details(session_id,title,archived) VALUES(?1,?2,1)",
            params![session.id, header["title"].as_str().unwrap_or("")],
        )?;
        loop {
            let line = read_line(&mut reader)?;
            if line.is_empty() {
                break;
            }
            bytes += line.len();
            ensure!(bytes <= 512 * 1024 * 1024, "Conversation archive too large");
            let value: serde_json::Value = serde_json::from_str(&line)?;
            tx.execute(
                "INSERT INTO events(session_id,payload) VALUES(?1,?2)",
                params![session.id, value.to_string()],
            )?;
        }
        tx.commit()?;
        Ok(session.id)
    }
}
