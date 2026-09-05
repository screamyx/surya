//! A2UI card rows: the glue between `surya-a2ui` and the transcript. Maps
//! comet's theme onto the card token set, loads demo fixtures, and prints
//! the render counter the proof reads (`asked=N rendered=N`).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use gpui::{Bounds, ListState, Pixels, SharedString, Window};
use surya_a2ui::{Card, CardTheme, ImagePolicy};

use crate::state::AppState;
use crate::theme::Theme;

/// The demo fixtures dir (`ZERON_DEMO_CARDS`), read once.
fn demo_dir() -> Option<&'static PathBuf> {
    static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
    DIR.get_or_init(|| {
        std::env::var_os("ZERON_DEMO_CARDS")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    })
    .as_ref()
}

/// Whether the render/measure counters print (`ZERON_DEMO_CARDS` or
/// `SURYA_CARD_STATS=1`); off by default so a card row costs no log line
/// and no per-frame closure.
fn stats_enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        demo_dir().is_some()
            || std::env::var("SURYA_CARD_STATS").is_ok_and(|v| !v.is_empty() && v != "0")
    })
}

/// Where the selected chat's cards may load images from: the chat's
/// working folder and the demo fixtures dir. Remote images stay off until
/// a setting exists for them.
pub fn image_policy(state: &AppState) -> ImagePolicy {
    let mut allowed_dirs: Vec<PathBuf> = state
        .selected_chat_row()
        .and_then(|c| c.cwd.clone())
        .map(PathBuf::from)
        .into_iter()
        .collect();
    if let Some(dir) = demo_dir() {
        allowed_dirs.push(dir.clone());
    }
    ImagePolicy {
        allowed_dirs,
        remote_allowed: false,
    }
}

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
        // Comet's code wash is the accent at low alpha; a card of paths in
        // that tint is a card of orange (critic round 2, C1). Every code
        // surface reads the neutral wash through one seam since PR #36, so
        // card code matches transcript code by construction.
        code_wash: crate::markdown::render::inline_code_wash(theme),
        input_bg: theme.input_bg,
        danger: theme.danger_muted,
        font_sans: theme.font_sans.clone(),
        font_mono: theme.font_mono.clone(),
        check_icon: Some(SharedString::from(crate::icons::CHECK)),
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
    let base = fixture_base(dir);
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

/// The `file://` prefix a fixture's `fixture://` becomes, escaped for a
/// JSON string: a Windows dir has backslashes, and pasting them raw made
/// every fixture with an image unparseable (dtry 07:52: card 01 never
/// seeded). Forward slashes work on Windows too.
pub fn fixture_base(dir: &Path) -> String {
    let dir = dir.display().to_string();
    // Only Windows separators are rewritten: a Unix dir name may legally
    // contain a backslash, and rewriting it would point at nothing.
    #[cfg(windows)]
    let dir = dir.replace('\\', "/");
    let base = format!("file://{dir}/");
    // serde's string escaping, minus the quotes it wraps.
    serde_json::to_string(&base)
        .map(|q| q[1..q.len() - 1].to_owned())
        .unwrap_or(base)
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
pub fn log_render(
    ix: usize,
    row_id: &SharedString,
    version: u64,
    card: &Card,
    list: &ListState,
    window: &mut Window,
) {
    if !stats_enabled() {
        return;
    }
    // Once per (row, card version, window width): the list repaints idle
    // rows every few hundred ms and the counter must not follow suit. A
    // new version of a row forgets the widths logged for the old one, so
    // the set is bounded by rows on screen times widths seen.
    thread_local! {
        static LOGGED: std::cell::RefCell<
            std::collections::HashMap<SharedString, (u64, std::collections::HashSet<i64>)>,
        > = Default::default();
    }
    let width = f32::from(window.viewport_size().width) as i64;
    let fresh = LOGGED.with(|l| {
        let mut map = l.borrow_mut();
        let entry = map.entry(row_id.clone()).or_insert((version, Default::default()));
        if entry.0 != version {
            *entry = (version, Default::default());
        }
        entry.1.insert(width)
    });
    if !fresh {
        return;
    }
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

    /// A fixture dir whose name holds a backslash and quotes still yields
    /// valid JSON, and the `file://` URL resolves to the file beside the
    /// fixture (on Unix the backslash is a real character and stays).
    #[test]
    fn fixture_base_survives_odd_dir_names_and_resolves() {
        let dir = std::env::temp_dir().join(format!("surya a2ui\\odd \"dir\" {}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("p.jpg"), b"jpg").unwrap();
        std::fs::write(dir.join("01.json"), r#"{"components":[{"id":"root","component":"Image","url":"fixture://p.jpg"}]}"#).unwrap();
        let loaded = load_fixture_dir(&dir);
        assert_eq!(loaded.len(), 1, "parsed despite the odd dir name");
        let url = loaded[0].1["components"][0]["url"].as_str().unwrap().to_owned();
        let path = url.strip_prefix("file://").expect("file url");
        assert!(Path::new(path).is_file(), "{url} does not resolve");
        assert_eq!(Path::new(path).canonicalize().unwrap(), dir.join("p.jpg").canonicalize().unwrap());
        // The policy accepts it under its own dir, so the card would load it.
        let policy = ImagePolicy { allowed_dirs: vec![dir.clone()], remote_allowed: false };
        assert!(matches!(policy.decide(&url), surya_a2ui::ImageDecision::File(_)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The hostile fixtures parse without panicking and land on the
    /// diagnostics the hardening added: the huge index is refused, the
    /// exfiltrating images never resolve to a fetch, the fan-out card is
    /// parse-clean (the render budget owns it, see `surya_a2ui::budget`).
    #[test]
    fn hostile_fixtures_degrade_to_diagnostics() {
        let dir = fixtures_dir().join("hostile");
        let cards = load_fixture_dir(&dir);
        assert_eq!(cards.len(), 4, "hostile fixtures present");
        let policy = ImagePolicy::default();
        let mut refused = 0;
        let mut blocked_images = 0;
        for (stem, json) in &cards {
            let card = surya_a2ui::parse_card(json);
            assert!(card.root().is_some(), "{stem}: root");
            if stem == "huge-index" {
                assert!(card.errors.iter().any(|e| e.contains("out of bounds")), "{stem}: {:?}", card.errors);
                refused += 1;
            }
            for c in card.components.values() {
                if let surya_a2ui::ComponentKind::Image { url: surya_a2ui::model::Dynamic::Literal(u), .. } = &c.kind {
                    match policy.decide(u) {
                        surya_a2ui::ImageDecision::File(_) | surya_a2ui::ImageDecision::Remote(_) => {
                            panic!("{stem}: {u} would load")
                        }
                        _ => blocked_images += 1,
                    }
                }
            }
        }
        eprintln!("hostile asked=4 parsed=4 index_refused={refused} images_blocked={blocked_images}");
        assert_eq!((refused, blocked_images), (1, 3));
    }

    trait Tap: Sized {
        fn tap(self, f: impl FnOnce(&Self)) -> Self {
            f(&self);
            self
        }
    }
    impl<T> Tap for T {}
}
