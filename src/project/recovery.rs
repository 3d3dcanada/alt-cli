//! Preserve the actual displaced file at the rename boundary. This coordinates Alt
//! writers and observes external writers; it is not a filesystem-wide CAS.
use super::*;
use std::fs::{File, OpenOptions};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileIdentity {
    pub device: u64,
    pub inode: u64,
    pub mode: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub temporary: String,
    pub expected_hash: Option<String>,
    pub intended_hash: Option<String>,
    pub expected_mode: u32,
    pub intended_identity: Option<FileIdentity>,
    pub operation: String,
}

pub(super) fn project_lock(root: &Path) -> Result<File> {
    #[cfg(unix)]
    let owner = unsafe { nix::libc::geteuid() };
    #[cfg(not(unix))]
    let owner = std::env::var("USERNAME").unwrap_or_else(|_| "user".into());
    let directory = std::env::temp_dir().join(format!("alt-project-locks-{owner}"));
    if !directory.exists() {
        match std::fs::create_dir(&directory) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
    }
    let metadata = std::fs::symlink_metadata(&directory)?;
    ensure!(
        metadata.is_dir() && !metadata.file_type().is_symlink(),
        "Project coordination directory is not an ordinary private directory"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        ensure!(
            metadata.uid() == owner,
            "Project coordination directory belongs to another user"
        );
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
    }
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(nix::libc::O_NOFOLLOW);
    }
    let file = options.open(directory.join(digest(root.to_string_lossy().as_bytes())))?;
    file.try_lock_exclusive().context("Another Alt action is using this project (possibly in another data folder); wait for it to finish")?;
    Ok(file)
}

/// Linux renameat2 retains the displaced inode, or installs without replacement.
/// Unsupported filesystems fail before an ordinary overwriting rename is attempted.
#[cfg(target_os = "linux")]
fn rename_guarded(dir: &Dir, from: &Path, to: &Path, exchange: bool) -> Result<()> {
    use std::os::{fd::AsRawFd, unix::ffi::OsStrExt};
    let from = std::ffi::CString::new(from.as_os_str().as_bytes())?;
    let to = std::ffi::CString::new(to.as_os_str().as_bytes())?;
    let flags = if exchange {
        nix::libc::RENAME_EXCHANGE
    } else {
        nix::libc::RENAME_NOREPLACE
    };
    let result = unsafe {
        nix::libc::renameat2(
            dir.as_raw_fd(),
            from.as_ptr(),
            dir.as_raw_fd(),
            to.as_ptr(),
            flags,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context("Atomic retained-file replacement is unavailable or the destination changed; no ordinary overwriting fallback was used");
    }
    Ok(())
}
#[cfg(not(target_os = "linux"))]
fn rename_guarded(_: &Dir, _: &Path, _: &Path, _: bool) -> Result<()> {
    bail!("Tracked atomic replacement currently requires Linux; Full terminal remains available")
}

impl Project {
    pub(super) fn file_identity(&self, path: &str) -> Result<Option<FileIdentity>> {
        self.check_path(path)?;
        match self.dir.metadata(path) {
            Ok(metadata) => {
                ensure!(metadata.is_file(), "Tracked changes require ordinary files");
                #[cfg(unix)]
                {
                    use cap_std::fs::{MetadataExt, PermissionsExt};
                    Ok(Some(FileIdentity {
                        device: metadata.dev(),
                        inode: metadata.ino(),
                        mode: metadata.permissions().mode() & 0o777,
                    }))
                }
                #[cfg(not(unix))]
                {
                    Ok(Some(FileIdentity {
                        device: 0,
                        inode: 0,
                        mode: 0,
                    }))
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    pub(super) fn ensure_identity(&self, c: &Change, undo: bool) -> Result<()> {
        let expected_hash = if undo { &c.after_hash } else { &c.before_hash };
        let expected = if undo {
            &c.after_identity
        } else {
            &c.before_identity
        };
        let actual = self.file_identity(&c.path)?;
        if expected_hash.is_none() {
            ensure!(
                actual.is_none(),
                "File was created by another writer; inspect it before continuing"
            );
        } else if let Some(expected) = expected {
            ensure!(
                actual.as_ref() == Some(expected)
                    && expected.mode
                        == if undo {
                            c.after_mode.unwrap_or(c.mode)
                        } else {
                            c.mode
                        },
                "File identity or permissions changed after this action was prepared; your version was preserved. Read and prepare a fresh edit"
            );
        } else {
            // Legacy checkpoints lack identity but still must not widen permissions.
            ensure!(
                actual.as_ref().is_some_and(|m| m.mode
                    == if undo {
                        c.after_mode.unwrap_or(c.mode)
                    } else {
                        c.mode
                    }),
                "File permissions changed; reconcile them before applying or undoing this legacy checkpoint"
            );
        }
        Ok(())
    }
    fn temporary_bytes(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let mut options = cap_std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            options.custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK);
        }
        let mut file = match self.dir.open_with(path, &options) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let metadata = file.metadata()?;
        ensure!(
            metadata.is_file() && metadata.len() <= PROJECT_LIMIT,
            "Displaced source is larger than the recovery capture limit; retained in project temporary file"
        );
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(PROJECT_LIMIT + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= PROJECT_LIMIT,
            "Displaced source grew beyond recovery capture limit; temporary file retained"
        );
        Ok(Some(bytes))
    }
    fn retain_displaced(&self, c: &Change, transition: &Transition) -> Result<Option<String>> {
        let Some(bytes) = self.temporary_bytes(&transition.temporary)? else {
            return Ok(None);
        };
        let hash = digest(&bytes);
        let destination = self
            .state
            .join("displaced")
            .join(&c.id)
            .join(&transition.operation);
        crate::config::private_dir(&destination)?;
        let blob = destination.join(format!("{hash}.source"));
        if !blob.exists() {
            crate::config::atomic_write(&blob, &bytes)?;
        }
        let mode = {
            #[cfg(unix)]
            {
                use cap_std::fs::PermissionsExt;
                self.dir
                    .metadata(&transition.temporary)?
                    .permissions()
                    .mode()
                    & 0o777
            }
            #[cfg(not(unix))]
            {
                0
            }
        };
        let record = serde_json::json!({"schema":1,"change":c.id,"path":c.path,"operation":transition.operation,"sha256":hash,"bytes":bytes.len(),"mode":mode,"expected_hash":transition.expected_hash,"temporary":transition.temporary,"guarantee":"Observed displaced inode captured after atomic rename; external in-place writers are not coordinated"});
        crate::config::atomic_write(
            &destination.join(format!("{hash}.json")),
            &serde_json::to_vec_pretty(&record)?,
        )?;
        Ok(Some(hash))
    }
    pub(super) fn write_image(&self, c: &mut Change, image: Option<&str>) -> Result<()> {
        self.check_path(&c.path)?;
        let undo = c.status == "undoing";
        let expected_mode = if undo {
            c.after_mode.unwrap_or(c.mode)
        } else {
            c.mode
        };
        let intended_mode = if undo {
            c.mode
        } else {
            c.after_mode.unwrap_or(c.mode)
        };
        let expected_hash = if undo {
            c.after_hash.clone()
        } else {
            c.before_hash.clone()
        };
        let parent = Path::new(&c.path).parent().unwrap_or(Path::new(""));
        if !parent.as_os_str().is_empty() {
            self.dir.create_dir_all(parent)?;
        }
        let directory = self.dir.open_dir(if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        })?;
        let target = Path::new(&c.path)
            .file_name()
            .context("Missing source filename")?;
        let temporary_name = format!(".alt-write-{}-{}", c.id, uuid::Uuid::new_v4());
        let temporary = parent.join(&temporary_name).to_string_lossy().into_owned();
        let intended_identity = if let Some(text) = image {
            let mut options = cap_std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            let mut file = directory.open_with(&temporary_name, &options)?;
            file.write_all(text.as_bytes())?;
            #[cfg(unix)]
            {
                use cap_std::fs::PermissionsExt;
                file.set_permissions(cap_std::fs::Permissions::from_mode(intended_mode))?;
            }
            file.sync_all()?;
            #[cfg(unix)]
            {
                use cap_std::fs::{MetadataExt, PermissionsExt};
                let m = file.metadata()?;
                Some(FileIdentity {
                    device: m.dev(),
                    inode: m.ino(),
                    mode: m.permissions().mode() & 0o777,
                })
            }
            #[cfg(not(unix))]
            {
                Some(FileIdentity {
                    device: 0,
                    inode: 0,
                    mode: 0,
                })
            }
        } else {
            None
        };
        let transition = Transition {
            temporary,
            expected_hash: expected_hash.clone(),
            intended_hash: image.map(|s| digest(s.as_bytes())),
            expected_mode,
            intended_identity,
            operation: if undo { "undo" } else { "apply" }.into(),
        };
        c.transition = Some(transition.clone());
        self.save_change(c)?;
        if self.current_hash(&c.path)? != expected_hash || self.ensure_identity(c, undo).is_err() {
            c.status = "conflict".into();
            self.save_change(c)?;
            if image.is_some() {
                let _ = directory.remove_file(&temporary_name);
            }
            bail!(
                "File changed while staging this action; external version preserved. Read and prepare a fresh edit"
            );
        }
        if image.is_none() {
            rename_guarded(
                &directory,
                Path::new(target),
                Path::new(&temporary_name),
                false,
            )?;
        } else {
            rename_guarded(
                &directory,
                Path::new(&temporary_name),
                Path::new(target),
                expected_hash.is_some(),
            )?;
        }
        // Sync the rename before recording completion. On crash, the journal identifies
        // both the intended inode and the retained displaced inode without replaying edits.
        directory.open(".")?.sync_all()?;
        if expected_hash.is_some() {
            let observed = self.retain_displaced(c, &transition)?;
            let mode = {
                #[cfg(unix)]
                {
                    use cap_std::fs::PermissionsExt;
                    self.dir
                        .metadata(&transition.temporary)?
                        .permissions()
                        .mode()
                        & 0o777
                }
                #[cfg(not(unix))]
                {
                    0
                }
            };
            if observed != expected_hash || mode != expected_mode {
                c.status = "conflict".into();
                self.save_change(c)?;
                bail!(
                    "An external edit raced with replacement. Its displaced version is retained under {} and {}; inspect both versions before continuing",
                    self.state.join("displaced").display(),
                    transition.temporary
                );
            }
        }
        ensure!(
            self.current_hash(&c.path)? == transition.intended_hash
                && self.file_identity(&c.path)? == transition.intended_identity,
            "Another writer changed the new source; retained checkpoint versions remain available"
        );
        // Status and intended inode identity are one durable record. A crash may
        // leave a staging file, but can never fall back to hash-only finalization.
        c.status = if undo { "undone" } else { "applied" }.into();
        if !undo {
            c.after_identity = transition.intended_identity.clone();
        }
        c.transition = None;
        let transaction = self.db.unchecked_transaction()?;
        self.save_change(c)?;
        if undo {
            self.rebind_previous_after_undo(c, &transition.intended_identity)?;
        }
        transaction.commit()?;
        if expected_hash.is_some() {
            directory.remove_file(&temporary_name)?;
            directory.open(".")?.sync_all()?;
        }
        Ok(())
    }
    pub(super) fn recover_transition(&self, c: &mut Change) -> Result<bool> {
        let Some(t) = c.transition.clone() else {
            return Ok(false);
        };
        let (current, identity) = match self
            .current_hash(&c.path)
            .and_then(|hash| Ok((hash, self.file_identity(&c.path)?)))
        {
            Ok(observation) => observation,
            Err(error) => {
                return self.recovery_conflict(
                    c,
                    &format!("Source cannot be inspected safely after interruption: {error:#}"),
                );
            }
        };
        let reached = current == t.intended_hash && identity == t.intended_identity;
        if reached {
            if t.expected_hash.is_some() {
                let observation = (|| -> Result<_> {
                    let displaced = self.retain_displaced(c, &t)?;
                    let displaced_mode = {
                        #[cfg(unix)]
                        {
                            use cap_std::fs::PermissionsExt;
                            self.dir.metadata(&t.temporary)?.permissions().mode() & 0o777
                        }
                        #[cfg(not(unix))]
                        {
                            0
                        }
                    };
                    Ok((displaced, displaced_mode))
                })();
                let (displaced, displaced_mode) =
                    match observation {
                        Ok(value) => value,
                        Err(error) => return self.recovery_conflict(
                            c,
                            &format!(
                                "Displaced file is missing or cannot be safely captured: {error:#}"
                            ),
                        ),
                    };
                if displaced != t.expected_hash || displaced_mode != t.expected_mode {
                    c.status = "conflict".into();
                    self.save_change(c)?;
                    return Ok(true);
                }
            }
            c.status = if t.operation == "apply" {
                "applied"
            } else {
                "undone"
            }
            .into();
            if t.operation == "apply" {
                c.after_identity = identity;
            }
        } else if current == t.expected_hash {
            c.status = if t.operation == "apply" {
                "proposed"
            } else {
                "applied"
            }
            .into();
        } else {
            // Do not overwrite any external post-crash change.
            let _ = self.retain_displaced(c, &t);
            c.status = "conflict".into();
        }
        let transaction = self.db.unchecked_transaction()?;
        self.save_change(c)?;
        if c.status == "undone" {
            self.rebind_previous_after_undo(c, &t.intended_identity)?;
        }
        transaction.commit()?;
        // Recovery leaves staging files as explicit evidence. Never delete an inode
        // based solely on matching content after an interrupted operation.
        Ok(true)
    }
    fn recovery_conflict(&self, c: &mut Change, message: &str) -> Result<bool> {
        c.status = "conflict".into();
        self.save_change(c)?;
        self.phase(
            &c.task,
            "Review",
            "Inspect recovered file versions; no action was replayed",
        )?;
        self.note(&c.task, "failure", &bounded(message, 2000), &c.id)?;
        Ok(true)
    }
    fn rebind_previous_after_undo(
        &self,
        undone: &Change,
        intended_identity: &Option<FileIdentity>,
    ) -> Result<()> {
        if let Some(mut previous) = self
            .changes(None)?
            .into_iter()
            .find(|c| c.id != undone.id && c.path == undone.path && c.status == "applied")
            && previous.after_hash == undone.before_hash
            && self.current_hash(&undone.path)? == undone.before_hash
            && self.file_identity(&undone.path)? == *intended_identity
        {
            previous.after_identity = intended_identity.clone();
            self.save_change(&previous)?;
        }
        Ok(())
    }
    /// Build a normal, reviewable tracked edit from one retained version. Applying
    /// it later uses the same current-content/identity checks as every other edit.
    pub fn prepare_displaced_restore(
        &self,
        change: &str,
        operation: &str,
        sha256: &str,
    ) -> Result<Change> {
        ensure!(
            matches!(operation, "apply" | "undo"),
            "Choose a recorded apply or undo version"
        );
        ensure!(
            sha256.len() == 64 && sha256.bytes().all(|c| c.is_ascii_hexdigit()),
            "Choose a valid retained SHA-256"
        );
        let original = self.change(change)?;
        let directory = self
            .state
            .join("displaced")
            .join(&original.id)
            .join(operation);
        let receipt: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join(format!("{sha256}.json")))?)?;
        ensure!(
            receipt["path"] == original.path && receipt["sha256"] == sha256,
            "Recovery receipt does not match this file"
        );
        let mut bytes = Vec::new();
        File::open(directory.join(format!("{sha256}.source")))?
            .take(FILE_LIMIT + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= FILE_LIMIT,
            "Retained version exceeds tracked-edit size; export it for manual recovery"
        );
        ensure!(
            digest(&bytes) == sha256,
            "Retained file digest changed; original receipt preserved"
        );
        let replacement = String::from_utf8(bytes)
            .context("Retained version is binary; export it for manual recovery")?;
        let current = self
            .bytes(&original.path)?
            .map(String::from_utf8)
            .transpose()?;
        if current.is_some() {
            self.read(&original.task, &original.path, 1, 300)?;
        }
        let mut proposal = self.prepare_edit(
            &original.task,
            &original.path,
            None,
            current.as_deref().unwrap_or(""),
            &replacement,
            if current.is_some() {
                "replace"
            } else {
                "create"
            },
            &format!("Restore retained {operation} version {sha256} from change {change}"),
        )?;
        let retained_mode = receipt["mode"].as_u64().unwrap_or(0o600) as u32 & 0o777;
        if current.is_none() {
            proposal.mode = retained_mode;
        } else {
            // Restoring content must not silently widen either the current or the
            // retained version's permissions. Ordinary Undo retains both modes.
            proposal.after_mode = Some(proposal.mode & retained_mode);
        }
        self.save_change(&proposal)?;
        Ok(proposal)
    }
    pub fn displaced_versions(&self) -> Result<Vec<serde_json::Value>> {
        let root = self.state.join("displaced");
        let mut output = Vec::new();
        if !root.exists() {
            return Ok(output);
        }
        for change in std::fs::read_dir(&root)? {
            let change = change?;
            if !change.file_type()?.is_dir() {
                continue;
            }
            for operation in std::fs::read_dir(change.path())? {
                let operation = operation?;
                if !operation.file_type()?.is_dir() {
                    continue;
                }
                for entry in std::fs::read_dir(operation.path())? {
                    let entry = entry?;
                    if entry.path().extension().and_then(|s| s.to_str()) != Some("json") {
                        continue;
                    }
                    ensure!(
                        output.len() < 20_000,
                        "Recovery catalog is too large; archive old project state"
                    );
                    let mut record: serde_json::Value =
                        serde_json::from_slice(&std::fs::read(entry.path())?)?;
                    record["receipt_path"] = serde_json::json!(entry.path());
                    output.push(record);
                }
            }
        }
        Ok(output)
    }
}
