//! A2UI card rows: the glue between `surya-a2ui` and the transcript. Maps
//! comet's theme onto the card token set, loads demo fixtures, and prints
//! the render counter the proof reads (`asked=N rendered=N`).

use std::path::Path;

use gpui::{Bounds, ListState, Pixels, SharedString, Window};
use surya_a2ui::{Card, CardTheme};

use crate::theme::Theme;

/// The card token set for the current theme. Numbers are comet's layout
/// constants; every color is a transcript token, so a card sits on the
/// same plane as the tool chips beside it.
pub fn card_theme(theme: &Theme) -> CardTheme {
    CardTheme {
        surface: theme.surface_card,
        surface_raised: theme.surface_raised,
        element_hover: theme.element_hover,
        border: theme.border,
        border_strong: theme.border_strong,
        text: theme.text,
        text_muted: theme.text_muted,
        text_faint: theme.text_faint,
        solid: theme.solid,
        on_solid: theme.on_solid,
        accent: theme.accent,
        code_text: theme.code_text,
        code_wash: theme.code_wash,
        input_bg: theme.input_bg,
        danger: theme.danger_muted,
        font_sans: theme.font_sans.clone(),
        font_mono: theme.font_mono.clone(),
        radius: Theme::PANEL_RADIUS,
        control_radius: Theme::CONTROL_RADIUS,
    }
}

/// Every `*.json` card in `dir`, sorted by file name, as `(stem, json)`.
/// `fixture://name` URLs become `file://<dir>/name` so a fixture can carry
/// an image beside it without an absolute path in the JSON.
pub fn load_fixture_dir(dir: &Path) -> Vec<(String, serde_json::Value)> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = read
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    let base = format!("file://{}/", dir.display());
    paths
        .into_iter()
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let text = text.replace("fixture://", &base);
            let json = serde_json::from_str(&text).ok()?;
            let stem = path.file_stem()?.to_string_lossy().into_owned();
            Some((stem, json))
        })
        .collect()
}

/// Demo entries for `ZERON_DEMO_CARDS=<dir>`: one assistant turn per
/// fixture, a one-line reply then the card (decision 14: "Reply text stays
/// to one line when a card is shown").
pub fn demo_entries(dir: &Path) -> Vec<zeron_doc::SessionMessageEntry> {
    let now = chrono::Utc::now().timestamp_millis();
    load_fixture_dir(dir)
        .into_iter()
        .enumerate()
        .map(|(ix, (stem, json))| zeron_doc::SessionMessageEntry {
            id: format!("demo-card-{stem}"),
            role: zeron_doc::MessageRole::Assistant,
            parts: vec![
                zeron_doc::MessagePart::Text {
                    id: "t0".into(),
                    text: format!("Card {}: `{stem}`", ix + 1),
                },
                zeron_doc::MessagePart::Card {
                    id: format!("toolu_{stem}"),
                    card_id: format!("card-{stem}"),
                    surface_id: json
                        .as_array()
                        .and_then(|l| l.first())
                        .and_then(|e| e["createSurface"]["surfaceId"].as_str())
                        .unwrap_or_default()
                        .to_owned(),
                    a2ui: Some(match json {
                        serde_json::Value::Array(items) => items,
                        other => vec![other],
                    }),
                    a2ui_ref: None,
                    a2ui_bytes: None,
                },
            ],
            created_at: now + ix as i64,
            device_id: "local".into(),
            status: Some(zeron_doc::MessageStatus::Complete),
            continuation_of: None,
        })
        .collect()
}

/// The proof counter: one line per card row render, then one more with the
/// measured row bounds once the list has laid it out. Read the log with
/// `grep 'card rendered'` and `grep 'card measured'`.
pub fn log_render(ix: usize, row_id: &SharedString, card: &Card, list: &ListState, window: &mut Window) {
    tracing::info!(
        target: "surya_a2ui",
        row = ix,
        row_id = %row_id,
        card = %card.id,
        components = card.components.len(),
        errors = card.errors.len(),
        "card rendered"
    );
    let list = list.clone();
    let row_id = row_id.clone();
    let card_id = card.id.clone();
    window.on_next_frame(move |window, _cx| {
        let viewport = window.viewport_size();
        if let Some(Bounds { size, .. }) = list.bounds_for_item(ix) {
            let (w, h): (Pixels, Pixels) = (size.width, size.height);
            tracing::info!(
                target: "surya_a2ui",
                row = ix,
                row_id = %row_id,
                card = %card_id,
                width = f32::from(w),
                height = f32::from(h),
                window_width = f32::from(viewport.width),
                "card measured"
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcript::{RowKind, rows_for_entry};

    fn fixtures_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../a2ui/fixtures")
            .canonicalize()
            .expect("fixtures dir")
    }

    /// The six mockup cards, translated to A2UI, each become one Card row
    /// with a parsed root and no diagnostics: `asked=6 parsed=6`.
    #[test]
    fn six_fixture_cards_become_six_clean_card_rows() {
        let entries = demo_entries(&fixtures_dir());
        let asked = entries.len();
        assert_eq!(asked, 6, "fixtures present");
        let mut parsed = 0;
        let mut parse = |key: &str, text: &str| {
            std::sync::Arc::new(crate::markdown::parser::parse_full(text))
                .tap(|_| assert!(!key.is_empty()))
        };
        for entry in &entries {
            let rows = rows_for_entry(entry, false, &mut parse);
            let cards: Vec<_> = rows
                .iter()
                .filter_map(|r| match &r.kind {
                    RowKind::Card { card, .. } => Some(card.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(cards.len(), 1, "{}: one card row", entry.id);
            let card = &cards[0];
            assert!(card.root().is_some(), "{}: has root", entry.id);
            assert!(card.errors.is_empty(), "{}: errors {:?}", entry.id, card.errors);
            // A text row precedes the card (the one-line reply).
            assert!(matches!(rows[0].kind, RowKind::Markdown { .. }), "{}: reply row first", entry.id);
            parsed += 1;
        }
        eprintln!("fixture cards asked={asked} parsed={parsed}");
        assert_eq!(parsed, asked);
    }

    /// Every catalog component the renderer names appears at least once
    /// across the fixtures, so the proof exercises all eleven.
    #[test]
    fn fixtures_cover_all_eleven_components() {
        let mut seen = std::collections::BTreeSet::new();
        for (_, json) in load_fixture_dir(&fixtures_dir()) {
            let card = surya_a2ui::parse_card(&json);
            for c in card.components.values() {
                seen.insert(c.kind.name().to_owned());
            }
        }
        for name in [
            "Text", "Image", "Button", "TextField", "CheckBox", "Row", "Column", "List", "Card",
            "Divider", "Tabs",
        ] {
            assert!(seen.contains(name), "{name} missing; seen {seen:?}");
        }
        eprintln!("components asked=11 covered={}", seen.len().min(11));
    }

    trait Tap: Sized {
        fn tap(self, f: impl FnOnce(&Self)) -> Self {
            f(&self);
            self
        }
    }
    impl<T> Tap for T {}
}
