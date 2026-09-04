//! Proof for the file RPCs against a temp git checkout: tree speed on a
//! 2k-file repo, watch on external edits, stale-hash refusal, jail refusal.
//! Every counter prints as a pair.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use futures::StreamExt as _;
use zeron_engine::files::{self, Jail};
use zeron_engine::files_watch;
use zeron_proto::files::{FileEventKind, FileKind, FileRead, FileWrite, LineRange};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .expect("git");
    assert!(status.success(), "git {args:?}");
}

/// A committed repo: `src/main.rs`, `src/deep/lib.rs`, an ignored `target/`.
fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let d = dir.path();
    std::fs::create_dir_all(d.join("src/deep")).unwrap();
    std::fs::create_dir_all(d.join("target")).unwrap();
    git(d, &["init", "-q"]);
    git(d, &["config", "user.email", "t@t"]);
    git(d, &["config", "user.name", "t"]);
    std::fs::write(d.join(".gitignore"), "target/\n").unwrap();
    std::fs::write(d.join("src/main.rs"), "fn main() {}\n").unwrap();
    std::fs::write(d.join("src/deep/lib.rs"), "pub fn x() {}\n").unwrap();
    std::fs::write(d.join("target/out.bin"), [0u8, 159, 146]).unwrap();
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "one"]);
    dir
}

fn jail(dir: &tempfile::TempDir) -> Jail {
    Jail::new(dir.path()).expect("jail")
}

#[test]
fn tree_lists_tracked_and_new_skips_ignored_and_carries_status() {
    let dir = repo();
    let d = dir.path();
    std::fs::write(d.join("src/main.rs"), "fn main() { changed }\n").unwrap();
    std::fs::write(d.join("notes.md"), "new\n").unwrap();
    let j = jail(&dir);

    // Depth 1: the root's children only; `src` says it has children.
    let top = files::tree(&j, "", 1).unwrap();
    let paths: Vec<&str> = top.entries.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(paths, vec![".gitignore", "notes.md", "src"]);
    let src = top.entries.iter().find(|e| e.path == "src").unwrap();
    assert_eq!(src.kind, FileKind::Dir);
    assert!(src.has_children);
    assert_eq!(
        src.status, "M",
        "a directory carries the strongest status under it"
    );
    assert_eq!(
        top.entries
            .iter()
            .find(|e| e.path == "notes.md")
            .unwrap()
            .status,
        "?"
    );
    assert!(!paths.contains(&"target"), "ignored dirs stay invisible");
    assert!(!paths.contains(&".git"));

    // Lazy expand of `src`.
    let sub = files::tree(&j, "src", 1).unwrap();
    let sp: Vec<&str> = sub.entries.iter().map(|e| e.path.as_str()).collect();
    assert_eq!(sp, vec!["src/deep", "src/main.rs"]);
    assert_eq!(
        sub.entries
            .iter()
            .find(|e| e.path == "src/main.rs")
            .unwrap()
            .status,
        "M"
    );
    assert_eq!(
        sub.entries
            .iter()
            .find(|e| e.path == "src/deep")
            .unwrap()
            .status,
        ""
    );
    println!(
        "tree: asked=2 answered=2 entries_top={} entries_src={}",
        top.entries.len(),
        sub.entries.len()
    );
}

#[test]
fn tree_of_2k_file_repo_under_100ms_after_warm_up() {
    let dir = repo();
    let d = dir.path();
    // 40 dirs x 50 files = 2000 files, plus 200 ignored ones.
    for i in 0..40 {
        let sub = d.join(format!("pkg{i:02}/src"));
        std::fs::create_dir_all(&sub).unwrap();
        for k in 0..50 {
            std::fs::write(sub.join(format!("mod{k:02}.rs")), "pub fn f() {}\n").unwrap();
        }
    }
    for k in 0..200 {
        std::fs::write(d.join(format!("target/obj{k}.o")), b"\0\0").unwrap();
    }
    git(d, &["add", "-A"]);
    git(d, &["commit", "-q", "-m", "two"]);
    let j = jail(&dir);

    let warm = files::tree(&j, "", 8).unwrap();
    let files_seen = warm
        .entries
        .iter()
        .filter(|e| e.kind == FileKind::File)
        .count();
    assert!(files_seen >= 2000, "files={files_seen}");
    assert!(!warm.entries.iter().any(|e| e.path.starts_with("target")));
    assert!(!warm.truncated);

    let mut samples = Vec::new();
    for _ in 0..5 {
        let t0 = Instant::now();
        let tree = files::tree(&j, "", 8).unwrap();
        samples.push(t0.elapsed());
        assert_eq!(tree.entries.len(), warm.entries.len());
    }
    samples.sort();
    let median = samples[samples.len() / 2];
    println!(
        "tree 2k: files={files_seen} entries={} runs={} median_ms={:.1} min_ms={:.1} max_ms={:.1}",
        warm.entries.len(),
        samples.len(),
        median.as_secs_f64() * 1e3,
        samples[0].as_secs_f64() * 1e3,
        samples[samples.len() - 1].as_secs_f64() * 1e3,
    );
    assert!(median < Duration::from_millis(100), "median {median:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn watch_emits_on_external_edits() {
    let dir = repo();
    let d = dir.path().to_path_buf();
    let j = jail(&dir);
    let mut batches = files_watch::watch(j).expect("watch");
    // Let the watcher settle before the first edit.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let target = d.join("src/main.rs");
    let mut events_seen = 0usize;
    let edits = 3usize;
    for i in 0..edits {
        std::fs::write(&target, format!("fn main() {{ edit {i} }}\n")).unwrap();
        // Something in `target/` should never show up.
        std::fs::write(d.join("target/noise.o"), b"\0").unwrap();
        let batch = tokio::time::timeout(Duration::from_secs(3), batches.next())
            .await
            .expect("a batch within 3s")
            .expect("stream open");
        assert_eq!(batch.seq as usize, i + 1);
        assert!(!batch.truncated);
        assert!(
            batch.events.iter().any(|e| e.path == "src/main.rs"
                && matches!(e.kind, FileEventKind::Modify | FileEventKind::Create)),
            "batch {i}: {:?}",
            batch.events
        );
        assert!(
            !batch.events.iter().any(|e| e.path.starts_with("target")),
            "ignored paths leaked: {:?}",
            batch.events
        );
        events_seen += 1;
        // Space the edits past the debounce so each is its own batch.
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    println!("watch: edits={edits} events={events_seen}");
    assert_eq!(events_seen, edits);

    // Create + delete arrive with their kinds.
    let fresh = d.join("src/new.rs");
    std::fs::write(&fresh, "x\n").unwrap();
    let batch = tokio::time::timeout(Duration::from_secs(3), batches.next())
        .await
        .unwrap()
        .unwrap();
    assert!(
        batch
            .events
            .iter()
            .any(|e| e.path == "src/new.rs" && e.kind == FileEventKind::Create),
        "{:?}",
        batch.events
    );
    tokio::time::sleep(Duration::from_millis(250)).await;
    std::fs::remove_file(&fresh).unwrap();
    let batch = tokio::time::timeout(Duration::from_secs(3), batches.next())
        .await
        .unwrap()
        .unwrap();
    assert!(
        batch
            .events
            .iter()
            .any(|e| e.path == "src/new.rs" && e.kind == FileEventKind::Delete),
        "{:?}",
        batch.events
    );
    drop(batches);
}

#[test]
fn read_gives_hash_ranges_and_detects_binary() {
    let dir = repo();
    let j = jail(&dir);
    let FileRead::Text {
        text,
        hash,
        size,
        lines,
    } = files::read(&j, "src/main.rs", None).unwrap()
    else {
        panic!("text")
    };
    assert_eq!(text, "fn main() {}\n");
    assert_eq!(size, 13);
    assert_eq!(lines, 1);
    assert_eq!(hash, files::content_hash(b"fn main() {}\n"));

    std::fs::write(dir.path().join("many.txt"), "a\nb\nc\nd\n").unwrap();
    let FileRead::Text { text, lines, .. } =
        files::read(&j, "many.txt", Some(LineRange { start: 1, end: 3 })).unwrap()
    else {
        panic!("text")
    };
    assert_eq!(text, "b\nc\n");
    assert_eq!(lines, 4);

    // Ignored by git, but a direct read by path still works: the tree hides
    // it, the jail does not.
    assert_eq!(
        files::read(&j, "target/out.bin", None).unwrap(),
        FileRead::Binary { size: 3 }
    );
    assert!(
        files::read(&j, "src", None).is_err(),
        "a directory is not readable"
    );
    println!("read: asked=4 answered=4");
}

#[test]
fn write_with_stale_hash_is_refused_and_fresh_hash_saves() {
    let dir = repo();
    let j = jail(&dir);
    let f = dir.path().join("src/main.rs");
    let FileRead::Text { hash: h1, .. } = files::read(&j, "src/main.rs", None).unwrap() else {
        panic!("text")
    };
    // Somebody else (an agent) writes first.
    std::fs::write(&f, "fn main() { agent }\n").unwrap();
    let mut asked = 0;
    let mut refused = 0;
    asked += 1;
    match files::write(&j, "src/main.rs", "fn main() { me }\n", Some(&h1)).unwrap() {
        FileWrite::Refused { reason, text, hash } => {
            refused += 1;
            assert!(reason.contains("changed on disk"), "{reason}");
            assert_eq!(text.as_deref(), Some("fn main() { agent }\n"));
            assert_eq!(
                hash.as_deref(),
                Some(files::content_hash(b"fn main() { agent }\n").as_str())
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(&f).unwrap(),
        "fn main() { agent }\n",
        "nothing clobbered"
    );
    println!("write stale: asked={asked} refused={refused}");
    assert_eq!((asked, refused), (1, 1));

    // With the current hash it saves, and the hash moves on.
    let FileRead::Text { hash: h2, .. } = files::read(&j, "src/main.rs", None).unwrap() else {
        panic!("text")
    };
    let FileWrite::Saved { hash: h3, size } =
        files::write(&j, "src/main.rs", "fn main() { me }\n", Some(&h2)).unwrap()
    else {
        panic!("saved")
    };
    assert_ne!(h2, h3);
    assert_eq!(size, 17);
    assert_eq!(std::fs::read_to_string(&f).unwrap(), "fn main() { me }\n");

    // Create needs no hash; creating over an existing file is refused; a
    // hash against a missing file is refused.
    assert!(matches!(
        files::write(&j, "src/fresh.rs", "x\n", None).unwrap(),
        FileWrite::Saved { .. }
    ));
    assert!(matches!(
        files::write(&j, "src/fresh.rs", "y\n", None).unwrap(),
        FileWrite::Refused { .. }
    ));
    assert!(
        matches!(files::write(&j, "src/gone.rs", "y\n", Some("00")).unwrap(), FileWrite::Refused { reason, .. } if reason.contains("gone"))
    );

    // CRLF on disk stays CRLF.
    let win = dir.path().join("win.txt");
    std::fs::write(&win, "a\r\nb\r\n").unwrap();
    let FileRead::Text { hash, .. } = files::read(&j, "win.txt", None).unwrap() else {
        panic!()
    };
    files::write(&j, "win.txt", "a\nb\nc\n", Some(&hash)).unwrap();
    assert_eq!(std::fs::read(&win).unwrap(), b"a\r\nb\r\nc\r\n");
    // No temp file left behind.
    let leftovers: Vec<PathBuf> = std::fs::read_dir(dir.path().join("src"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().unwrap().to_string_lossy().contains(".zeron-"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[test]
fn paths_outside_the_root_are_refused() {
    let dir = repo();
    let j = jail(&dir);
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.txt"), "s\n").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        outside.path().join("secret.txt"),
        dir.path().join("link.txt"),
    )
    .unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), dir.path().join("linkdir")).unwrap();

    let mut cases: Vec<String> = vec![
        "../secret.txt".into(),
        "src/../../secret.txt".into(),
        outside
            .path()
            .join("secret.txt")
            .to_string_lossy()
            .into_owned(),
        "/etc/hostname".into(),
    ];
    #[cfg(unix)]
    {
        cases.push("link.txt".into());
        cases.push("linkdir/secret.txt".into());
        cases.push("linkdir/new.txt".into());
    }
    let asked = cases.len();
    let mut refused = 0;
    for case in &cases {
        let read = files::read(&j, case, None);
        let write = files::write(&j, case, "pwned\n", None);
        let tree = files::tree(&j, case, 1);
        if read.is_err() && write.is_err() && tree.is_err() {
            refused += 1;
        } else {
            panic!("{case}: read={read:?} write={write:?} tree={tree:?}");
        }
    }
    assert_eq!(
        std::fs::read_to_string(outside.path().join("secret.txt")).unwrap(),
        "s\n"
    );
    println!("jail: asked={asked} refused={refused}");
    assert_eq!(asked, refused);

    // The one case that must pass: a plain in-root path, dotted segments and all.
    assert!(files::read(&j, "./src/./main.rs", None).is_ok());
    assert!(
        files::tree(&j, "/", 1).is_ok(),
        "a leading slash is tolerated as root-relative"
    );
}
