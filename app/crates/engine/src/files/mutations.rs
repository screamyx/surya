//! File actions from the tree, jailed to the selected checkout.

use super::{Jail, other};
use crate::EngineError;
use std::path::{Component, Path, PathBuf};
use surya_proto::files::FileMutation;

fn target(jail: &Jail, path: &str) -> Result<PathBuf, EngineError> {
    if path.is_empty() || path.contains(['\\', ':']) || Path::new(path).is_absolute() {
        return Err(other("Choose a relative file path inside the project."));
    }
    let mut lexical = jail.root().to_path_buf();
    for component in Path::new(path).components() {
        let Component::Normal(name) = component else {
            return Err(other("Choose a relative file path inside the project."));
        };
        if name.to_string_lossy().eq_ignore_ascii_case(".git") {
            return Err(other("Git metadata cannot be changed from the file tree."));
        }
        lexical.push(name);
        if std::fs::symlink_metadata(&lexical).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Err(other("File actions do not follow symbolic links."));
        }
    }
    let resolved = jail.resolve(path)?;
    if resolved == jail.root() {
        return Err(other("The project root cannot be changed."));
    }
    Ok(resolved)
}

fn taken(rel: &str) -> EngineError {
    other(format!("{rel} already exists. Choose another name."))
}

/// Create never overwrites; rename reserves its destination without replacing
/// another file; delete only removes one regular file, never a directory tree.
pub fn mutate(jail: &Jail, action: &FileMutation) -> Result<(), EngineError> {
    match action {
        FileMutation::Create { path: rel } => {
            let path = target(jail, rel)?;
            if let Err(err) = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                return Err(match err.kind() {
                    std::io::ErrorKind::AlreadyExists => taken(rel),
                    std::io::ErrorKind::NotFound => {
                        other("That folder is not in the project any more.")
                    }
                    _ => err.into(),
                });
            }
        }
        FileMutation::Rename { path, new_path } => {
            let from = target(jail, path)?;
            let to = target(jail, new_path)?;
            if !std::fs::symlink_metadata(&from)?.is_file() {
                return Err(other("Choose a regular file to rename."));
            }
            // hard_link fails if the destination already exists on both
            // Windows and Unix. std::fs::rename may replace it on Unix.
            if let Err(err) = std::fs::hard_link(&from, &to) {
                return Err(match err.kind() {
                    std::io::ErrorKind::AlreadyExists => taken(new_path),
                    _ => err.into(),
                });
            }
            if let Err(err) = std::fs::remove_file(&from) {
                let _ = std::fs::remove_file(&to);
                return Err(err.into());
            }
        }
        FileMutation::Delete { path } => {
            let path = target(jail, path)?;
            if !std::fs::symlink_metadata(&path)?.is_file() {
                return Err(other("Choose a regular file to delete."));
            }
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_rename_delete_preserve_bytes_and_refuse_existing_destinations() {
        let dir = tempfile::tempdir().unwrap();
        let jail = Jail::new(dir.path()).unwrap();
        mutate(
            &jail,
            &FileMutation::Create {
                path: "notes.txt".into(),
            },
        )
        .unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"keep me").unwrap();
        assert!(
            mutate(
                &jail,
                &FileMutation::Create {
                    path: "notes.txt".into()
                }
            )
            .is_err()
        );
        std::fs::write(dir.path().join("taken.txt"), b"other").unwrap();
        assert!(
            mutate(
                &jail,
                &FileMutation::Rename {
                    path: "notes.txt".into(),
                    new_path: "taken.txt".into()
                }
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read(dir.path().join("taken.txt")).unwrap(),
            b"other"
        );
        mutate(
            &jail,
            &FileMutation::Rename {
                path: "notes.txt".into(),
                new_path: "renamed.txt".into(),
            },
        )
        .unwrap();
        assert!(!dir.path().join("notes.txt").exists());
        assert_eq!(
            std::fs::read(dir.path().join("renamed.txt")).unwrap(),
            b"keep me"
        );
        mutate(
            &jail,
            &FileMutation::Delete {
                path: "renamed.txt".into(),
            },
        )
        .unwrap();
        assert!(!dir.path().join("renamed.txt").exists());
    }

        #[test]
    fn a_taken_name_is_refused_in_words_not_an_errno() {
        let dir = tempfile::tempdir().unwrap();
        let jail = Jail::new(dir.path()).unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"one").unwrap();
        let err = mutate(
            &jail,
            &FileMutation::Create {
                path: "notes.txt".into(),
            },
        )
        .unwrap_err()
        .to_string();
        assert_eq!(err, "notes.txt already exists. Choose another name.");
    }

#[test]
    fn actions_refuse_outside_paths_roots_directories_and_git_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let jail = Jail::new(dir.path()).unwrap();
        // A real checkout always has .git, and the guard is the only thing
        // stopping a forwarded FilesMutate from writing into it. Without
        // these two lines every .git assert below passes on ENOENT from the
        // missing parent instead of on the guard.
        std::fs::create_dir_all(dir.path().join(".git/hooks")).unwrap();
        std::fs::write(dir.path().join(".git/config"), b"[core]").unwrap();
        for path in [
            "",
            ".",
            "../outside",
            "/outside",
            "C:\\outside",
            ".git/config",
            ".git/hooks/pre-commit",
        ] {
            assert!(
                mutate(&jail, &FileMutation::Create { path: path.into() }).is_err(),
                "{path}"
            );
        }
        // Deleting inside .git is the clearest signal: with the guard gone the
        // file exists and is regular, so remove_file succeeds and both of
        // these fail.
        assert!(
            mutate(
                &jail,
                &FileMutation::Delete {
                    path: ".git/config".into()
                }
            )
            .is_err()
        );
        assert!(dir.path().join(".git/config").exists());
        std::fs::create_dir(dir.path().join("folder")).unwrap();
        assert!(
            mutate(
                &jail,
                &FileMutation::Delete {
                    path: "folder".into()
                }
            )
            .is_err()
        );
        assert!(
            mutate(
                &jail,
                &FileMutation::Rename {
                    path: "folder".into(),
                    new_path: "other".into()
                }
            )
            .is_err()
        );
        assert!(dir.path().join("folder").is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn actions_never_follow_file_or_directory_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let jail = Jail::new(dir.path()).unwrap();
        std::fs::write(dir.path().join("real.txt"), b"preserve").unwrap();
        std::os::unix::fs::symlink("real.txt", dir.path().join("link.txt")).unwrap();
        assert!(
            mutate(
                &jail,
                &FileMutation::Delete {
                    path: "link.txt".into()
                }
            )
            .is_err()
        );
        assert!(
            mutate(
                &jail,
                &FileMutation::Rename {
                    path: "link.txt".into(),
                    new_path: "other".into()
                }
            )
            .is_err()
        );
        std::os::unix::fs::symlink(dir.path(), dir.path().join("linked-dir")).unwrap();
        assert!(
            mutate(
                &jail,
                &FileMutation::Create {
                    path: "linked-dir/other".into()
                }
            )
            .is_err()
        );
        assert_eq!(
            std::fs::read(dir.path().join("real.txt")).unwrap(),
            b"preserve"
        );
    }
}
