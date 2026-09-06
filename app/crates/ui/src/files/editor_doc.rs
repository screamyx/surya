//! The editor's pure state: haktui `editor.rs::Tab` without the entity.
//! What a read becomes, when a save may go, what a refusal does. No gpui,
//! so every rule here is unit-tested.

use surya_proto::files::{FileRead, FileWrite};

/// What the editor shows for a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Empty,
    Loading,
    Text,
    Binary { size: u64 },
    TooLarge { size: u64, max: u64 },
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub reason: String,
    pub text: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved,
    Refused,
}

/// What a click on a tree path does to the buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Switch {
    /// Replace the buffer with the path's read.
    Load,
    /// The buffer holds edits for another path: ask before anything moves.
    Prompt,
    /// The same dirty file was clicked again: nothing to do.
    Stay,
}

/// Pure editor state: haktui `editor.rs::Tab` without the entity.
#[derive(Debug, Default)]
pub struct EditorDoc {
    pub path: Option<String>,
    pub body: Body,
    /// The content hash the buffer was read with; `FilesWrite` wants it back.
    pub hash: Option<String>,
    /// The text as of the last read or save; the buffer against it is dirty.
    pub saved_text: String,
    pub conflict: Option<Conflict>,
    pub saving: bool,
    pub note: Option<String>,
    pub saves_asked: u32,
    pub saved: u32,
    pub refused: u32,
    /// A path waiting behind the unsaved-edits prompt.
    pub pending_open: Option<String>,
}

impl Default for Body {
    fn default() -> Self {
        Body::Empty
    }
}

impl EditorDoc {
    pub fn loading(&mut self, path: &str) {
        self.path = Some(path.to_string());
        self.body = Body::Loading;
        self.hash = None;
        self.conflict = None;
        self.note = None;
        self.saving = false;
        // A prompt was about the buffer being replaced; it does not follow
        // the next file.
        self.pending_open = None;
    }

    /// A read answered. Returns the text to put in the buffer, if any.
    pub fn opened(&mut self, path: &str, read: FileRead) -> Option<String> {
        if self.path.as_deref() != Some(path) {
            return None; // a stale answer for a path we left
        }
        match read {
            FileRead::Text { text, hash, .. } => {
                self.body = Body::Text;
                self.hash = Some(hash);
                self.saved_text = text.clone();
                Some(text)
            }
            FileRead::Binary { size } => {
                self.body = Body::Binary { size };
                None
            }
            FileRead::TooLarge { size, max } => {
                self.body = Body::TooLarge { size, max };
                None
            }
        }
    }

    pub fn failed(&mut self, why: String) {
        self.saving = false;
        self.pending_open = None;
        if self.body == Body::Loading {
            self.body = Body::Error(why);
        } else {
            self.note = Some(why);
        }
    }

    pub fn is_dirty(&self, current: &str) -> bool {
        self.body == Body::Text && current != self.saved_text
    }

    /// The rule for a click on `path` while the buffer holds `current`:
    /// a dirty buffer is never replaced without asking, and the same dirty
    /// file clicked again is left alone. A clean buffer loads, the same path
    /// included (a re-click is a refresh from disk).
    pub fn switch_to(&self, current: &str, path: &str) -> Switch {
        if !self.is_dirty(current) {
            return Switch::Load;
        }
        if self.path.as_deref() == Some(path) {
            Switch::Stay
        } else {
            Switch::Prompt
        }
    }

    /// A click, applied: the rule from `switch_to`, and its state change.
    /// `Prompt` remembers the path behind the prompt; `Stay` takes any
    /// prompt down (clicking the file you are on is choosing to stay);
    /// `Load` is the caller's to do.
    pub fn click(&mut self, current: &str, path: &str) -> Switch {
        let switch = self.switch_to(current, path);
        match switch {
            Switch::Prompt => self.ask_before_leaving(path),
            Switch::Stay => self.keep_editing(),
            Switch::Load => {}
        }
        switch
    }

    /// Whether the prompt is up: a path waits, and the buffer is still
    /// dirty. Edited back to clean, the question is moot and the bar goes.
    pub fn prompt_for(&self, current: &str) -> Option<&str> {
        if !self.is_dirty(current) {
            return None;
        }
        self.pending_open.as_deref()
    }

    /// Remember the path the prompt is about.
    pub fn ask_before_leaving(&mut self, path: &str) {
        self.pending_open = Some(path.to_string());
    }

    /// The prompt's Discard, or the save that the prompt's Save asked for
    /// just landed: the waiting path, to load now.
    pub fn take_pending_open(&mut self) -> Option<String> {
        self.pending_open.take()
    }

    /// The prompt's Keep editing: nothing moves.
    pub fn keep_editing(&mut self) {
        self.pending_open = None;
    }

    /// Whether a save may go now, and with which expected hash. Nothing goes
    /// while a conflict banner is up: the owner decides first.
    pub fn save_request(&mut self, current: &str) -> Option<(String, Option<String>)> {
        let path = self.path.clone()?;
        if self.body != Body::Text || self.saving || self.conflict.is_some() {
            return None;
        }
        if !self.is_dirty(current) {
            return None; // nothing to write
        }
        self.saving = true;
        self.saves_asked += 1;
        Some((path, self.hash.clone()))
    }

    /// `FilesWrite` answered for `text_at_save`.
    pub fn write_answered(&mut self, write: FileWrite, text_at_save: String) -> SaveOutcome {
        self.saving = false;
        match write {
            FileWrite::Saved { hash, .. } => {
                self.hash = Some(hash);
                self.saved_text = text_at_save;
                self.conflict = None;
                self.note = None;
                self.saved += 1;
                SaveOutcome::Saved
            }
            FileWrite::Refused { reason, text, hash } => {
                self.conflict = Some(Conflict { reason, text, hash });
                self.refused += 1;
                // The conflict banner takes over; the file the owner was
                // going to is not opened behind their back. They click it
                // again once the conflict is settled.
                self.pending_open = None;
                SaveOutcome::Refused
            }
        }
    }

    /// Take what is on disk: the conflict's text replaces the buffer and its
    /// hash becomes ours. Returns the text to load.
    pub fn take_theirs(&mut self) -> Option<String> {
        let c = self.conflict.take()?;
        let text = c.text.unwrap_or_default();
        self.hash = c.hash;
        self.saved_text = text.clone();
        Some(text)
    }

    /// Keep ours: the next save carries the disk's current hash, so the
    /// engine accepts it and the other writer's bytes are replaced.
    pub fn keep_mine(&mut self) -> bool {
        let Some(c) = self.conflict.take() else {
            return false;
        };
        if c.hash.is_some() {
            self.hash = c.hash;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_read(text: &str, hash: &str) -> FileRead {
        FileRead::Text {
            text: text.into(),
            hash: hash.into(),
            size: text.len() as u64,
            lines: text.lines().count() as u64,
        }
    }

    #[test]
    fn save_conflict_then_overwrite() {
        let mut doc = EditorDoc::default();
        doc.loading("src/main.rs");
        assert_eq!(
            doc.opened("src/main.rs", text_read("a\n", "h1")).as_deref(),
            Some("a\n")
        );
        assert!(!doc.is_dirty("a\n"));
        assert!(doc.is_dirty("ab\n"));

        // Save 1 goes through.
        let (path, expected) = doc.save_request("ab\n").expect("save allowed");
        assert_eq!(
            (path.as_str(), expected.as_deref()),
            ("src/main.rs", Some("h1"))
        );
        assert!(
            doc.save_request("ab\n").is_none(),
            "one save in flight at a time"
        );
        assert_eq!(
            doc.write_answered(
                FileWrite::Saved {
                    hash: "h2".into(),
                    size: 3
                },
                "ab\n".into()
            ),
            SaveOutcome::Saved
        );
        assert!(!doc.is_dirty("ab\n"));

        // Somebody else wrote; save 2 is refused and the banner is up.
        let _ = doc.save_request("abc\n").unwrap();
        assert_eq!(
            doc.write_answered(
                FileWrite::Refused {
                    reason: "changed on disk since it was read".into(),
                    text: Some("theirs\n".into()),
                    hash: Some("h3".into()),
                },
                "abc\n".into()
            ),
            SaveOutcome::Refused
        );
        assert!(doc.conflict.is_some());
        assert!(
            doc.save_request("abc\n").is_none(),
            "no save while the banner is up"
        );

        // Overwrite: keep mine, the disk's hash rides the next save.
        assert!(doc.keep_mine());
        let (_, expected) = doc.save_request("abc\n").unwrap();
        assert_eq!(expected.as_deref(), Some("h3"));
        assert_eq!(
            doc.write_answered(
                FileWrite::Saved {
                    hash: "h4".into(),
                    size: 4
                },
                "abc\n".into()
            ),
            SaveOutcome::Saved
        );
        println!(
            "editor: saves_asked={} saved={} refused={}",
            doc.saves_asked, doc.saved, doc.refused
        );
        assert_eq!((doc.saves_asked, doc.saved, doc.refused), (3, 2, 1));
    }

    #[test]
    fn a_dirty_buffer_is_never_replaced_without_asking() {
        let mut doc = EditorDoc::default();
        assert_eq!(doc.switch_to("", "a.rs"), Switch::Load, "nothing open: load");
        doc.loading("a.rs");
        doc.opened("a.rs", text_read("one\n", "h1"));
        assert_eq!(doc.switch_to("one\n", "b.rs"), Switch::Load, "clean: load");
        assert_eq!(doc.switch_to("one\n", "a.rs"), Switch::Load, "clean re-click: refresh");
        assert_eq!(doc.switch_to("one two\n", "b.rs"), Switch::Prompt, "dirty, other file: ask");
        assert_eq!(doc.switch_to("one two\n", "a.rs"), Switch::Stay, "dirty, same file: nothing");

        // Discard: the waiting path loads, the edit is gone with it.
        doc.ask_before_leaving("b.rs");
        assert_eq!(doc.take_pending_open().as_deref(), Some("b.rs"));
        assert_eq!(doc.take_pending_open(), None, "taken once");

        // Keep editing: nothing waits any more.
        doc.ask_before_leaving("b.rs");
        doc.keep_editing();
        assert_eq!(doc.take_pending_open(), None);

        // Save then open: the write lands, the waiting path is still there.
        doc.ask_before_leaving("b.rs");
        let _ = doc.save_request("one two\n").expect("save allowed");
        assert_eq!(
            doc.write_answered(FileWrite::Saved { hash: "h2".into(), size: 8 }, "one two\n".into()),
            SaveOutcome::Saved
        );
        assert_eq!(doc.take_pending_open().as_deref(), Some("b.rs"));

        // Save refused: the conflict banner takes over and nothing opens.
        doc.ask_before_leaving("b.rs");
        let _ = doc.save_request("one three\n").expect("save allowed");
        assert_eq!(
            doc.write_answered(
                FileWrite::Refused { reason: "changed".into(), text: None, hash: None },
                "one three\n".into()
            ),
            SaveOutcome::Refused
        );
        assert_eq!(doc.take_pending_open(), None, "a refused save does not open the next file");

        // A placeholder body is never dirty, so a binary file switches freely.
        doc.loading("blob.bin");
        doc.opened("blob.bin", FileRead::Binary { size: 3 });
        assert_eq!(doc.switch_to("", "a.rs"), Switch::Load);
    }

    #[test]
    fn the_prompt_lives_only_as_long_as_the_dirty_buffer() {
        let mut doc = EditorDoc::default();
        doc.loading("a.rs");
        doc.opened("a.rs", text_read("one\n", "h1"));

        // Dirty, click b: the prompt is up for b.
        assert_eq!(doc.click("one!\n", "b.rs"), Switch::Prompt);
        assert_eq!(doc.prompt_for("one!\n"), Some("b.rs"));
        // Backspace to clean: the bar goes, though the path is still remembered.
        assert_eq!(doc.prompt_for("one\n"), None);
        // Clean, click c: it loads, and the stale path does not follow it.
        assert_eq!(doc.click("one\n", "c.rs"), Switch::Load);
        doc.loading("c.rs");
        assert_eq!(doc.pending_open, None, "loading() clears the pending path");
        doc.opened("c.rs", text_read("see\n", "h2"));
        assert_eq!(doc.prompt_for("see!\n"), None, "dirty again, but nothing waits");

        // Dirty, click b, then click c again while the prompt is up: the
        // prompt moves to c. Click c itself (the file we are on): Stay takes
        // the prompt down.
        assert_eq!(doc.click("see!\n", "b.rs"), Switch::Prompt);
        assert_eq!(doc.click("see!\n", "a.rs"), Switch::Prompt);
        assert_eq!(doc.prompt_for("see!\n"), Some("a.rs"));
        assert_eq!(doc.click("see!\n", "c.rs"), Switch::Stay);
        assert_eq!(doc.pending_open, None, "Stay takes the prompt down");
        assert_eq!(doc.prompt_for("see!\n"), None);
    }

    #[test]
    fn reload_takes_theirs_and_binary_gets_a_placeholder() {
        let mut doc = EditorDoc::default();
        doc.loading("x");
        doc.opened("x", text_read("mine\n", "h1"));
        doc.save_request("mine2\n");
        doc.write_answered(
            FileWrite::Refused {
                reason: "changed".into(),
                text: Some("theirs\n".into()),
                hash: Some("h9".into()),
            },
            "mine2\n".into(),
        );
        assert_eq!(doc.take_theirs().as_deref(), Some("theirs\n"));
        assert_eq!(doc.hash.as_deref(), Some("h9"));
        assert!(!doc.is_dirty("theirs\n"));

        doc.loading("blob.bin");
        assert!(
            doc.opened("blob.bin", FileRead::Binary { size: 9 })
                .is_none()
        );
        assert_eq!(doc.body, Body::Binary { size: 9 });
        assert!(
            doc.save_request("anything").is_none(),
            "no saves for a placeholder"
        );
        // A stale answer for a path we left is ignored.
        assert!(doc.opened("x", text_read("late\n", "h0")).is_none());
        assert_eq!(doc.body, Body::Binary { size: 9 });
    }
}
