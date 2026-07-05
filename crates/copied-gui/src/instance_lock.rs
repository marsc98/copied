use std::io;
use std::path::{Path, PathBuf};

pub enum LockOutcome {
    Acquired(LockGuard),
    SignaledExisting,
}

pub struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

pub fn acquire_or_signal_existing() -> io::Result<LockOutcome> {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "XDG_RUNTIME_DIR não está setada; não é possível localizar o lock file",
        )
    })?;
    let path = Path::new(&runtime_dir).join("copied-gui.pid");
    acquire_or_signal_existing_at(&path)
}

fn acquire_or_signal_existing_at(path: &Path) -> io::Result<LockOutcome> {
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            if let Some(pid) = contents
                .trim()
                .parse::<i32>()
                .ok()
                .filter(|pid| is_alive(*pid))
            {
                unsafe {
                    libc::kill(pid, libc::SIGUSR1);
                }
                return Ok(LockOutcome::SignaledExisting);
            }
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(err),
    }

    std::fs::write(path, std::process::id().to_string())?;
    Ok(LockOutcome::Acquired(LockGuard {
        path: path.to_path_buf(),
    }))
}

fn is_alive(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_succeeds_when_no_lock_exists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("copied-gui.pid");

        let outcome = acquire_or_signal_existing_at(&path).unwrap();

        assert!(matches!(outcome, LockOutcome::Acquired(_)));
        assert!(path.exists());
    }

    #[test]
    fn signals_existing_when_pid_alive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("copied-gui.pid");
        let mut child = std::process::Command::new("sleep")
            .arg("5")
            .spawn()
            .unwrap();
        std::fs::write(&path, child.id().to_string()).unwrap();

        let outcome = acquire_or_signal_existing_at(&path).unwrap();

        assert!(matches!(outcome, LockOutcome::SignaledExisting));
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn acquires_when_lock_file_is_stale() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("copied-gui.pid");
        let mut child = std::process::Command::new("true").spawn().unwrap();
        let dead_pid = child.id();
        child.wait().unwrap();
        std::fs::write(&path, dead_pid.to_string()).unwrap();

        let outcome = acquire_or_signal_existing_at(&path).unwrap();

        assert!(matches!(outcome, LockOutcome::Acquired(_)));
    }
}
