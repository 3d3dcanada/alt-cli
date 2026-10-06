//! Ownership helpers shared by the agent engine and local inference server.
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
