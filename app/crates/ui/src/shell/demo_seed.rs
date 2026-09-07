//! The capture knobs' fake chats: which one is on screen, and when a frame
//! is allowed to put it back.
//!
//! `SURYA_DEMO_CARDS` and `SURYA_DEMO_MAIL` each seed a chat the engine's
//! list does not carry, so every `apply_chats` drops it again. What decides
//! whether a frame re-inserts it, and whether that insert takes the window,
//! is one pure function with one table of cases.
//!
//! Its own file because `shell.rs` is far past the 500-line rule
//! (decision 13) and must not grow.

use crate::state::AppState;

/// The fake chat `SURYA_DEMO_CARDS` shows its fixtures in.
pub(super) const DEMO_CARDS_CHAT: &str = "demo-cards";

/// The fake chat `SURYA_DEMO_MAIL` shows its rows in.
pub(super) const DEMO_MAIL_CHAT: &str = crate::transcript::demo_mail::DEMO_MAIL_CHAT;

/// Put a capture knob's fake chat at the top of the sidebar, and give it the
/// window when this seed is the one that takes it.
///
/// The engine's chat list never carries these rows, so every `apply_chats`
/// drops them again; `demo_seed_step` decides per frame whether to re-insert.
pub(super) fn insert_demo_chat(
    s: &mut AppState,
    chat_id: &str,
    title: &str,
    entries: Option<Vec<surya_doc::SessionMessageEntry>>,
) {
    s.chats.insert(
        0,
        surya_proto::Chat {
            id: chat_id.into(),
            device_id: s.local_device_id.clone().unwrap_or_else(|| "local".into()),
            title: Some(title.into()),
            archived: false,
            cwd: None,
            branch: None,
            checkout_id: None,
            source_context: None,
            config: None,
            last_message_preview: None,
            last_message_at: None,
            created_at: chrono::Utc::now(),
            harness_session_id: None,
            harness_session_cwd: None,
            space_id: None,
            last_seen_at: None,
            room_gen: Default::default(),
        },
    );
    if let Some(entries) = entries {
        s.auto_selected = true;
        s.selected_chat = Some(chat_id.into());
        s.transcript = entries;
        s.transcript_replayed = true;
    }
}

/// What the `SURYA_DEMO_CARDS` knob does on one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DemoSeed {
    Skip,
    /// Put the demo row back; `select` also moves the window onto it.
    Insert { select: bool },
}

/// The demo row goes back only when the chat sync dropped it, and that seed
/// takes the window only on the first one, when the demo itself was
/// selected, or when nothing is selected and the user never pressed `+`:
/// a user who clicked another chat, or pressed `+`, keeps what they chose.
///
/// `chat_id` is the knob's own chat. It used to be the cards chat and only
/// the cards chat, which meant a second knob could never recognise its own
/// row as the selected one.
pub(super) fn demo_seed_step(
    chat_id: &str,
    synced: bool,
    seeded_before: bool,
    user_pressed_plus: bool,
    row_present: bool,
    selected: Option<&str>,
) -> DemoSeed {
    if !synced || row_present {
        return DemoSeed::Skip;
    }
    let select = !seeded_before
        || selected == Some(chat_id)
        || (selected.is_none() && !user_pressed_plus);
    DemoSeed::Insert { select }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_seed_re_arms_only_when_the_row_vanished() {
        use DemoSeed::*;
        // (synced, seeded_before, user_pressed_plus, row_present, selected) -> step,
        // all against DEMO_CARDS_CHAT as the knob's own chat.
        let cases = [
            ((false, false, false, false, None), Skip, "before the chat list lands"),
            ((true, false, false, false, Some("real")), Insert { select: true }, "first seed takes the window"),
            ((true, true, false, true, Some("real")), Skip, "user clicked another chat: leave it"),
            ((true, true, true, true, None), Skip, "user pressed +, row still there: leave it"),
            ((true, true, false, false, None), Insert { select: true }, "sync dropped row and selection: restore both"),
            ((true, true, false, false, Some("real")), Insert { select: false }, "sync dropped the row while the user is elsewhere: row only"),
            ((true, true, true, false, None), Insert { select: false }, "user pressed +, then a sync dropped the row: row only, canvas stays empty"),
            ((true, true, true, false, Some("demo-cards")), Insert { select: true }, "user came back to the demo after +, then a sync dropped it: restore"),
        ];
        let asked = cases.len();
        let mut passed = 0;
        for ((synced, seeded, plus, present, selected), want, why) in cases {
            assert_eq!(
                demo_seed_step(DEMO_CARDS_CHAT, synced, seeded, plus, present, selected),
                want,
                "{why}"
            );
            passed += 1;
        }
        eprintln!("demo seed cases asked={asked} passed={passed}");
        assert_eq!(passed, asked);
    }

    /// The nit that made the parameter: a second knob has to recognise its
    /// OWN row as the selected one, not the cards chat's.
    #[test]
    fn each_knob_reads_its_own_chat_as_the_selection() {
        assert_eq!(
            demo_seed_step(DEMO_MAIL_CHAT, true, true, false, false, Some(DEMO_MAIL_CHAT)),
            DemoSeed::Insert { select: true },
            "the mail knob's own chat was selected: restore it"
        );
        assert_eq!(
            demo_seed_step(DEMO_MAIL_CHAT, true, true, false, false, Some(DEMO_CARDS_CHAT)),
            DemoSeed::Insert { select: false },
            "another knob's chat is selected: put the row back, leave the window"
        );
    }
}
