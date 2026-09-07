//! Own a mail endpoint before recovering any stale socket left at its path.
use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;

/// Keep the returned file alive for as long as the listener. A separate lock
/// file remains on disk across restarts; unlinking it would let two processes
/// lock different inodes while claiming the same endpoint.
pub(super) fn bind(path: &Path) -> io::Result<(UnixListener, File)> {
    let mut lock_path = path.as_os_str().to_os_string();
    lock_path.push(".lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(lock_path)?;
    // SAFETY: the descriptor belongs to `lock`, which remains open here and
    // in the returned guard. flock neither reads nor writes Rust memory.
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        let error = io::Error::last_os_error();
        return Err(if error.kind() == io::ErrorKind::WouldBlock {
            io::Error::new(
                io::ErrorKind::AddrInUse,
                "another engine owns this mail socket",
            )
        } else {
            error
        });
    }

    let listener = match UnixListener::bind(path) {
        Ok(listener) => listener,
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => {
            // Older engines do not hold the lock. Probe their endpoint before
            // unlinking anything, and never replace a regular file or symlink.
            if !std::fs::symlink_metadata(path)?.file_type().is_socket() {
                return Err(error);
            }
            match UnixStream::connect(path) {
                Ok(_) => return Err(error),
                Err(probe) if probe.kind() == io::ErrorKind::ConnectionRefused => {
                    std::fs::remove_file(path)?;
                }
                Err(probe) => return Err(probe),
            }
            UnixListener::bind(path)?
        }
        Err(error) => return Err(error),
    };
    listener.set_nonblocking(true)?;
    Ok((listener, lock))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn second_listener_cannot_replace_a_live_endpoint() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.sock");
        let (first, _guard) = bind(&path).unwrap();
        let inode = std::fs::metadata(&path).unwrap().ino();
        assert_eq!(bind(&path).unwrap_err().kind(), io::ErrorKind::AddrInUse);
        assert_eq!(std::fs::metadata(&path).unwrap().ino(), inode);
        let _client = UnixStream::connect(&path).unwrap();
        first.accept().unwrap();
    }

    #[test]
    fn listener_from_an_older_engine_without_a_lock_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.sock");
        let first = UnixListener::bind(&path).unwrap();
        let inode = std::fs::metadata(&path).unwrap().ino();
        assert_eq!(bind(&path).unwrap_err().kind(), io::ErrorKind::AddrInUse);
        assert_eq!(std::fs::metadata(&path).unwrap().ino(), inode);
        first.accept().unwrap(); // the liveness probe reached the old listener
    }

    #[test]
    fn stale_socket_and_released_lock_allow_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.sock");
        drop(bind(&path).unwrap());
        assert!(path.exists());
        let (restarted, _guard) = bind(&path).unwrap();
        let _client = UnixStream::connect(&path).unwrap();
        restarted.accept().unwrap();
    }

    #[test]
    fn existing_non_socket_paths_are_never_removed() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("record");
        std::fs::write(&file, b"keep me").unwrap();
        assert!(bind(&file).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"keep me");
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        assert!(bind(&link).is_err());
        assert!(
            std::fs::symlink_metadata(link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn separate_paths_allow_independent_engines() {
        let dir = tempfile::tempdir().unwrap();
        let _first = bind(&dir.path().join("first.sock")).unwrap();
        let _second = bind(&dir.path().join("second.sock")).unwrap();
    }

    #[test]
    fn concurrent_starts_have_exactly_one_owner() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mail.sock");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let threads: Vec<_> = (0..2)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let result = bind(&path);
                    barrier.wait(); // keep the winner's guard until both attempted
                    result.is_ok()
                })
            })
            .collect();
        let owners = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(|owns| *owns)
            .count();
        assert_eq!(owners, 1);
    }
}
