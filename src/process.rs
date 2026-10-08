//! Ownership helpers shared by the agent engine and local inference server.
/// Keep the process group identity after the leader exits. A surviving helper
/// still belongs to this launch and must not escape cleanup merely because
/// `Child::id()` becomes None after wait/try_wait.
pub struct OwnedGroup {
    pid: Option<u32>,
    identity: Option<crate::jobs::Identity>,
}
impl OwnedGroup {
    pub fn capture(child: &tokio::process::Child) -> Self {
        let pid = child.id();
        Self {
            pid,
            identity: pid.and_then(crate::jobs::Identity::read),
        }
    }
    #[cfg(unix)]
    fn signal(&self, signal: nix::sys::signal::Signal) {
        if let Some(pid) = self.pid {
            // An extant leader with another start identity is a reused PID.
            // A dead leader is not evidence that its process group is empty.
            if let Some(current) = crate::jobs::Identity::read(pid)
                && self
                    .identity
                    .as_ref()
                    .is_some_and(|owned| owned != &current)
            {
                return;
            }
            let _ = nix::sys::signal::killpg(nix::unistd::Pid::from_raw(pid as i32), signal);
        }
    }
    pub fn kill(&self) {
        #[cfg(unix)]
        self.signal(nix::sys::signal::Signal::SIGKILL);
    }
    pub async fn stop(&mut self, child: &mut tokio::process::Child) {
        #[cfg(unix)]
        self.signal(nix::sys::signal::Signal::SIGTERM);
        // Always allow the group a grace interval, even if the leader has exited.
        let grace = tokio::time::sleep(std::time::Duration::from_millis(150));
        tokio::pin!(grace);
        tokio::select! { _ = child.wait() => {}, _ = &mut grace => {} }
        grace.await;
        self.kill();
        let _ = child.start_kill();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), child.wait()).await;
        self.pid = None;
    }
}
impl Drop for OwnedGroup {
    fn drop(&mut self) {
        self.kill();
    }
}

pub fn configure(command: &mut tokio::process::Command) {
    command.kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    #[cfg(target_os = "linux")]
    {
        let parent = nix::unistd::getpid();
        // Only async-signal-safe syscalls run between fork and exec. The parent
        // check closes the race where it exits before PR_SET_PDEATHSIG is set.
        unsafe {
            command.pre_exec(move || {
                nix::sys::prctl::set_pdeathsig(nix::sys::signal::Signal::SIGKILL)?;
                if nix::unistd::getppid() != parent {
                    return Err(std::io::Error::from_raw_os_error(
                        nix::errno::Errno::ECHILD as i32,
                    ));
                }
                Ok(())
            });
        }
    }
}

/// Short capability probes have the same ownership rules as long-running
/// engines. Bound both output pipes and stop surviving helpers after the probe
/// exits, times out, or fails to produce an acceptable result.
pub async fn bounded_output(
    command: &mut tokio::process::Command,
    deadline: std::time::Duration,
    max_bytes_per_pipe: usize,
) -> anyhow::Result<std::process::Output> {
    use tokio::io::AsyncReadExt;
    async fn read(
        pipe: impl tokio::io::AsyncRead + Unpin,
        limit: usize,
    ) -> anyhow::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        pipe.take(limit.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .await?;
        anyhow::ensure!(bytes.len() <= limit, "Probe output exceeded {limit} bytes");
        Ok(bytes)
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    configure(command);
    let mut child = command.spawn()?;
    let mut group = OwnedGroup::capture(&child);
    let stdout = child.stdout.take().expect("configured stdout pipe");
    let stderr = child.stderr.take().expect("configured stderr pipe");
    let result = tokio::time::timeout(deadline, async {
        let (stdout, stderr, status) = tokio::try_join!(
            read(stdout, max_bytes_per_pipe),
            read(stderr, max_bytes_per_pipe),
            async { Ok::<_, anyhow::Error>(child.wait().await?) }
        )?;
        Ok::<_, anyhow::Error>(std::process::Output {
            status,
            stdout,
            stderr,
        })
    })
    .await;
    group.stop(&mut child).await;
    result
        .map_err(|_| anyhow::anyhow!("Probe timed out after {} seconds", deadline.as_secs_f64()))?
}

pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        if let (Ok(mut terminate), Ok(mut hangup)) = (
            signal(SignalKind::terminate()),
            signal(SignalKind::hangup()),
        ) {
            tokio::select! {
                _=terminate.recv()=>{},
                _=hangup.recv()=>{},
                _=tokio::signal::ctrl_c()=>{},
            }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}
