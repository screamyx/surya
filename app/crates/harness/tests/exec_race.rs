//! Why the harness test fixtures are checked in and never written at run time.
//!
//! `crates/harness/tests/claude.rs` used to write a one-off replayer script
//! and exec it. On Linux CI that failed at about one run in three thousand
//! with `ExecutableFileBusy` ("Text file busy"), on a test that had nothing
//! to do with the change under review (issue #171).
//!
//! The cause is not two tests sharing a path. It is that `execve` refuses a
//! file that any process holds open for writing. A test binary runs its tests
//! on parallel threads: while one thread's write fd on the new script is still
//! open, another thread's `Command::spawn` forks, and the child inherits that
//! fd. `O_CLOEXEC` does not close it until the child reaches its OWN exec, so
//! for that window the script is "open for writing" and our exec of it fails.
//!
//! These tests pin the kernel behaviour the fix relies on, so nobody
//! reintroduces a written-then-exec'd fixture on the theory that a unique
//! path or a rename makes it safe. Neither does.
//!
//! The first test writes scripts and execs them, which is the pattern the
//! rest of this file argues against. It is one test rather than two on
//! purpose: a second forking test on a parallel thread could inherit its
//! write fd and hit the same ETXTBSY for real. The other test never forks,
//! so nothing here can race anything else here.

#![cfg(unix)]

use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

/// Write an executable script and hand back its path.
fn write_script(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, "#!/bin/sh\nexit 0\n").expect("script written");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

/// Both halves of the kernel behaviour the fix relies on, in ONE test.
///
/// Together on purpose. Each half writes a script and then execs it, which is
/// the pattern the rest of this file argues against, and two such tests on
/// parallel threads could inherit each other's write fd through a fork and
/// hit this same ETXTBSY for real.
#[test]
fn a_held_write_fd_blocks_exec_and_renaming_does_not_clear_it() {
    let dir = tempfile::tempdir().expect("tmp dir");

    // A held write fd is exactly what a sibling test's fork inherits, so this
    // reproduces the CI failure without waiting on the race to land.
    let script = write_script(dir.path(), "written.sh");

    // Exec is fine while nobody holds the file open for writing.
    Command::new(&script)
        .output()
        .expect("a script nobody is writing execs");

    let held = std::fs::OpenOptions::new()
        .write(true)
        .open(&script)
        .expect("open for write");
    let err = Command::new(&script)
        .output()
        .expect_err("exec must be refused while a write fd is open");
    assert_eq!(
        err.kind(),
        ErrorKind::ExecutableFileBusy,
        "expected ETXTBSY, got {err:?}"
    );

    // And it clears the moment the fd goes, which is why the failure is rare
    // and why rerunning the job always "fixes" it.
    drop(held);
    Command::new(&script)
        .output()
        .expect("execs again once the fd is closed");

    // The obvious workaround does not work, and issue #171 proposed it.
    //
    // `rename` moves a directory entry. The refusal is on the inode, and the
    // inode is the same one before and after, so a write fd opened on the old
    // name still blocks an exec of the new name. A unique path per test fails
    // for the same reason: the fd a sibling fork inherits has nothing to do
    // with which name we chose.
    let staged = write_script(dir.path(), "staged.sh.tmp");
    let held = std::fs::OpenOptions::new()
        .write(true)
        .open(&staged)
        .expect("open for write");

    let final_path = dir.path().join("staged.sh");
    std::fs::rename(&staged, &final_path).expect("rename");

    let err = Command::new(&final_path)
        .output()
        .expect_err("rename keeps the inode, so the block survives it");
    assert_eq!(
        err.kind(),
        ErrorKind::ExecutableFileBusy,
        "expected ETXTBSY after rename, got {err:?}"
    );

    drop(held);
    Command::new(&final_path)
        .output()
        .expect("execs once the fd is closed");
}

/// The fix: every program these tests exec is checked in, and no test opens
/// one for writing. If someone moves one back under a temp dir this stops
/// being true and this test says so.
#[test]
fn every_program_the_harness_tests_exec_is_checked_in() {
    let fixtures = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");

    // fake-claude.sh serves the replay scenario; the two under `acp/` are the
    // wedge and the config-option agent, both of which acp.rs used to write
    // into a temp dir and exec. The last three are exec'd straight from the
    // tree by codex.rs, cursor.rs, acp_quiet.rs and acp_stall.rs, which used
    // to chmod them on every call; this assertion is what replaces that.
    for name in [
        "fake-claude.sh",
        "acp/hung-agent.sh",
        "acp/devin.py",
        "fake-acp.sh",
        "fake-codex.sh",
        "fake-cursor-shim.sh",
    ] {
        let path = fixtures.join(name);
        assert!(path.is_file(), "{name} is checked in: {path:?}");
        assert!(
            path.metadata().expect("stat").permissions().mode() & 0o111 != 0,
            "{name} is executable in the tree, so no test has to chmod it"
        );
    }

    let capture = fixtures
        .join("claude")
        .join("live-2.1.228-background-subagent.jsonl");
    assert!(capture.is_file(), "the capture is checked in: {capture:?}");

    // The fake CLI knows the scenario the replay test asks for. Without this
    // the test would fall through to the unknown-scenario branch and still
    // "pass" a spawn, which is how a broken replay could look green.
    let body = std::fs::read_to_string(fixtures.join("fake-claude.sh")).expect("fake CLI readable");
    assert!(
        body.contains("*scenario:replay:*"),
        "the fake CLI serves scenario:replay"
    );
}
