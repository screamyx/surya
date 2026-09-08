//! Single-instance lock — an exclusive advisory `flock` on `{data_dir}/engine.lock`
//! held for the engine's lifetime. Two engines sharing one data dir would race the
//! SQLite snapshots DB and the append-only run journals (WAL + `busy_timeout` guard
//! individual statements, not whole-file ownership), so the second instance must
//! fail fast with a clear error instead of corrupting state.
//!
//! The lock is taken in `EngineCore::assemble_with_identity` BEFORE any store is opened
//! and before the IPC port binds, which also closes the race where a headed app's
//! TCP probe sees no daemon during another instance's startup window.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::EngineError;

/// Held lock on the data dir. Dropping it (engine shutdown / process exit)
/// releases the advisory lock; a crash releases it too (kernel-owned).
///
/// Release goes through an explicit `LOCK_UN`, not just the close in `Drop`.
/// A flock belongs to the OPEN FILE DESCRIPTION, so any process that inherited
/// this descriptor across a fork holds the lock alive until it closes it, and
/// closing our own copy is not enough. `LOCK_UN` on any one of those
/// descriptors drops the lock at once, which makes release deterministic.
#[derive(Debug)]
pub struct InstanceLock {
    // Read only by the unix `Drop` below; elsewhere it just has to stay open.
    #[cfg_attr(not(unix), allow(dead_code))]
    file: File,
}

impl InstanceLock {
    /// Acquire the exclusive lock, non-blocking. Errors with a descriptive
    /// message (including the holder's pid when readable) if another engine
    /// already owns this data dir.
    pub fn acquire(data_dir: &Path) -> Result<Self, EngineError> {
        let path = data_dir.join("engine.lock");
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;

        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            // Bounded EWOULDBLOCK retries: a fork→exec window in ANY process
            // that inherited the previous holder's fd (git scans, harness
            // spawns — fds are duplicated between fork and CLOEXEC-at-exec)
            // keeps the flock alive for a few milliseconds after release. A
            // real second engine holds it forever; transient artifacts clear
            // well within the budget.
            let mut retries = 40u32; // × 25ms = 1s budget
            loop {
                let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
                if rc == 0 {
                    break;
                }
                let errno = std::io::Error::last_os_error();
                match errno.raw_os_error() {
                    Some(libc::EINTR) => continue, // signal-interrupted: retry
                    Some(libc::EWOULDBLOCK) if retries > 0 => {
                        retries -= 1;
                        std::thread::sleep(std::time::Duration::from_millis(25));
                    }
                    Some(libc::EWOULDBLOCK) => {
                        let holder = std::fs::read_to_string(&path).unwrap_or_default();
                        let holder = holder.trim();
                        return Err(EngineError::Other(format!(
                            "another surya engine is already running on {} (pid {}); \
                             stop it or use a different data dir (SURYA_DATA_DIR)",
                            data_dir.display(),
                            if holder.is_empty() { "unknown" } else { holder },
                        )));
                    }
                    // Anything else (ENOLCK, filesystem without flock, …) is an
                    // environment problem, not a second engine — surface it as-is.
                    _ => return Err(EngineError::Io(errno)),
                }
            }
        }

        // Best-effort pid stamp for the contention error message above.
        let _ = file.set_len(0);
        let _ = write!(file, "{}", std::process::id());
        let _ = file.flush();
        Ok(Self { file })
    }

    /// Best-effort liveness probe: the pid stamped by the engine currently holding
    /// this data dir's lock, `None` when no engine is running (or the platform
    /// cannot test a lock without taking it). Used by `surya status` and the
    /// login/logout guards; a single non-blocking try — no retry budget — so a
    /// starting engine's transient fork-window artifacts read as "running", which
    /// is the safe direction for those callers.
    pub fn holder(data_dir: &Path) -> Option<String> {
        let path = data_dir.join("engine.lock");
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(&path)
                .ok()?;
            let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if rc == 0 {
                // We took it: nothing is running. Closing the fd releases it, but
                // unlock explicitly so the window is as small as possible.
                unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
                return None;
            }
            let pid = std::fs::read_to_string(&path).unwrap_or_default();
            let pid = pid.trim();
            Some(if pid.is_empty() {
                "unknown".to_string()
            } else {
                pid.to_string()
            })
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            None
        }
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            // Closing alone releases the lock only once EVERY descriptor for
            // this open file description is closed, and a fork in any thread
            // of this process duplicates it: `git` spawns, harness children,
            // anything between fork and its CLOEXEC-at-exec. Those copies kept
            // the lock alive for milliseconds after shutdown, so the next
            // engine start burned its retry budget against a dead one, and the
            // holder probe reported a pid that had already gone.
            //
            // `LOCK_UN` is not subject to that: it drops the lock from the
            // description itself, whoever else still has a handle on it.
            unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn holder_probe_reports_pid_without_disturbing_the_lock() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(InstanceLock::holder(dir.path()), None, "unlocked dir");
        let lock = InstanceLock::acquire(dir.path()).expect("acquire");
        assert_eq!(
            InstanceLock::holder(dir.path()).as_deref(),
            Some(std::process::id().to_string().as_str()),
        );
        // The probe must not have stolen the lock from the holder.
        InstanceLock::acquire(dir.path()).expect_err("still held after probe");
        drop(lock);
        assert_eq!(InstanceLock::holder(dir.path()), None, "released");
    }

    /// The flake this file was fixed for (issue #180), made deterministic.
    ///
    /// `dup` hands back a second descriptor for the same open file
    /// description, which is exactly what a fork leaves behind. With one of
    /// those outstanding, closing our own descriptor does NOT release the
    /// flock, so before the explicit `LOCK_UN` in `Drop` the probe below saw
    /// the lock still held and reported the pid of a lock that was gone.
    ///
    /// Measured on Linux with a standalone program mirroring acquire, release
    /// and probe. With a child forked between release and probe, close-only
    /// misreported a holder on the first cycle of every run; `LOCK_UN` then
    /// close ran 6000 cycles clean. Without the fork, close-only was clean
    /// too, which is what pins the cause on the inherited descriptor rather
    /// than on the close.
    #[test]
    fn dropping_the_lock_releases_it_even_with_a_forked_copy_of_the_fd() {
        use std::os::unix::io::AsRawFd;
        let dir = tempfile::tempdir().unwrap();
        let lock = InstanceLock::acquire(dir.path()).expect("acquire");
        // Stands in for the descriptor a concurrent fork inherited.
        let inherited = unsafe { libc::dup(lock.file.as_raw_fd()) };
        assert!(inherited >= 0, "dup failed");

        drop(lock);
        let holder = InstanceLock::holder(dir.path());
        unsafe { libc::close(inherited) };

        assert_eq!(
            holder, None,
            "a descriptor another process inherited must not keep the lock alive"
        );
    }

    #[test]
    fn second_acquire_fails_while_held_then_succeeds_after_drop() {
        let dir = tempfile::tempdir().unwrap();
        let lock = InstanceLock::acquire(dir.path()).expect("first acquire");
        let err = InstanceLock::acquire(dir.path()).expect_err("second acquire must fail");
        let msg = err.to_string();
        assert!(msg.contains("already running"), "unexpected error: {msg}");
        assert!(
            msg.contains(&std::process::id().to_string()),
            "holder pid missing from error: {msg}"
        );
        drop(lock);
        InstanceLock::acquire(dir.path()).expect("acquire after release");
    }
}
