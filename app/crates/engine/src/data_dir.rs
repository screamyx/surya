//! First-start adoption of an older data dir.
//!
//! The rename (decision 23) moves the data dir from `~/.zeron` to `~/.surya`.
//! A user who upgrades has their sign-in, device identity and preferences in
//! the old one, and starting fresh would silently sign them out.
//!
//! It **copies**, and never renames or deletes. A rename is atomic and cheap,
//! and it is what the 0.2.0 `.comet-native` migration did, but it leaves no
//! way back: a user who tries the new build and goes back to the old one finds
//! nothing. A copy costs disk once and keeps the rollback.
//!
//! Both names are parameters so the rename script only has to flip two
//! strings, and so this module says nothing about which release it serves.

use std::path::{Path, PathBuf};

/// What a start did about the previous data dir.
#[derive(Debug, PartialEq)]
pub enum Adoption {
    /// The current dir already exists. Every start after the first.
    AlreadyPresent,
    /// No previous dir to adopt. A machine that never ran the old build.
    NothingToAdopt,
    /// Copied, and here is where it came from.
    Copied(PathBuf),
    /// The copy failed. The caller carries on with an empty data dir rather
    /// than refusing to start, and the previous one is untouched.
    Failed(String),
}

/// Copy `home/previous` to `home/current` when the current dir is absent.
///
/// The copy lands in a sibling `<current>.incoming` first and is moved into
/// place with one rename. A copy that dies half way therefore leaves no
/// partial `<current>` behind - which would look adopted on the next start
/// and quietly strand the rest of the user's data.
pub fn adopt(home: &Path, current: &str, previous: &str) -> Adoption {
    let current = home.join(current);
    if current.exists() {
        return Adoption::AlreadyPresent;
    }
    let previous = home.join(previous);
    if !previous.is_dir() {
        return Adoption::NothingToAdopt;
    }
    let staging = current.with_extension("incoming");
    let _ = std::fs::remove_dir_all(&staging);
    if let Err(error) = copy_tree(&previous, &staging) {
        let _ = std::fs::remove_dir_all(&staging);
        return Adoption::Failed(format!("copying {previous:?}: {error}"));
    }
    if let Err(error) = std::fs::rename(&staging, &current) {
        let _ = std::fs::remove_dir_all(&staging);
        return Adoption::Failed(format!("moving the copy into {current:?}: {error}"));
    }
    Adoption::Copied(previous)
}

/// [`adopt`], reported to the user in one line. Returns whether it copied, so
/// a caller can count it.
pub fn adopt_and_report(home: &Path, current: &str, previous: &str) -> bool {
    match adopt(home, current, previous) {
        Adoption::Copied(from) => {
            let to = home.join(current);
            println!(
                "copied your data dir {} -> {} (the original is left in place)",
                from.display(),
                to.display()
            );
            true
        }
        Adoption::Failed(why) => {
            // Not fatal: an empty data dir starts, it just starts signed out.
            eprintln!("could not adopt the previous data dir ({why}); starting fresh");
            false
        }
        Adoption::AlreadyPresent | Adoption::NothingToAdopt => false,
    }
}

/// Recursive directory copy. Symlinks are skipped rather than followed: a data
/// dir is not expected to hold any, and following one can leave the tree or
/// loop forever.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = to.join(entry.file_name());
        if kind.is_symlink() {
            tracing::debug!(path = ?entry.path(), "skipping a symlink while adopting the data dir");
        } else if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn the_first_start_copies_and_the_second_does_not() {
        let home = tempfile::tempdir().unwrap();
        let home = home.path();
        write(&home.join(".zeron/ui-settings.json"), r#"{"theme":"dark"}"#);
        write(&home.join(".zeron/logs/engine.log"), "hello");

        let asked = 1;
        let copied = usize::from(adopt_and_report(home, ".surya", ".zeron"));
        assert_eq!((asked, copied), (1, 1), "asked={asked} copied={copied}");
        assert_eq!(
            std::fs::read_to_string(home.join(".surya/ui-settings.json")).unwrap(),
            r#"{"theme":"dark"}"#
        );
        assert_eq!(
            std::fs::read_to_string(home.join(".surya/logs/engine.log")).unwrap(),
            "hello",
            "nested files come too"
        );
        assert!(
            home.join(".zeron/ui-settings.json").exists(),
            "a COPY: the original is left for a rollback"
        );

        let copied_again = usize::from(adopt_and_report(home, ".surya", ".zeron"));
        assert_eq!((asked, copied_again), (1, 0), "second start copied={copied_again}");
    }

    #[test]
    fn a_machine_that_never_ran_the_old_build_is_left_alone() {
        let home = tempfile::tempdir().unwrap();
        assert_eq!(
            adopt(home.path(), ".surya", ".zeron"),
            Adoption::NothingToAdopt
        );
        assert!(!home.path().join(".surya").exists(), "nothing is created");
    }

    #[test]
    fn an_existing_current_dir_is_never_overwritten() {
        let home = tempfile::tempdir().unwrap();
        let home = home.path();
        write(&home.join(".zeron/ui-settings.json"), "old");
        write(&home.join(".surya/ui-settings.json"), "mine");
        assert_eq!(adopt(home, ".surya", ".zeron"), Adoption::AlreadyPresent);
        assert_eq!(
            std::fs::read_to_string(home.join(".surya/ui-settings.json")).unwrap(),
            "mine"
        );
    }

    /// The staging dir is the point: a half-copy must not look adopted.
    #[test]
    fn a_failed_copy_leaves_no_half_migrated_dir() {
        let home = tempfile::tempdir().unwrap();
        let home = home.path();
        write(&home.join(".zeron/ui-settings.json"), "old");
        // A FILE where the copy wants its staging directory, so create_dir_all
        // fails after the previous dir has been found.
        std::fs::write(home.join(".surya.incoming"), "in the way").unwrap();

        let outcome = adopt(home, ".surya", ".zeron");
        assert!(matches!(outcome, Adoption::Failed(_)), "{outcome:?}");
        assert!(!home.join(".surya").exists(), "no partial current dir");
        assert!(
            home.join(".zeron/ui-settings.json").exists(),
            "the previous dir is untouched"
        );
    }
}
