//! Renderer tests that build real elements without a window: gpui elements
//! are plain values until layout, so `Renderer::render` runs headless.

use std::rc::Rc;

use gpui::{SharedString, hsla};
use serde_json::json;

use crate::budget::Budget;
use crate::images::ImagePolicy;
use crate::parse::parse_card;
use crate::render::Renderer;
use crate::state::CardState;
use crate::theme::CardTheme;

fn theme() -> CardTheme {
    let c = hsla(0.0, 0.0, 0.5, 1.0);
    CardTheme {
        surface: c,
        surface_raised: c,
        element_hover: c,
        border: c,
        border_strong: c,
        text: c,
        text_muted: c,
        text_faint: c,
        solid: c,
        on_solid: c,
        accent: c,
        code_wash: c,
        input_bg: c,
        danger: c,
        font_sans: "sans".into(),
        font_mono: "mono".into(),
        check_icon: None,
        radius: 10.0,
        control_radius: 6.0,
    }
}

fn render_with(card_json: serde_json::Value, cap: usize) -> (Budget, usize) {
    let card = parse_card(&card_json);
    let state = CardState::new(&card);
    let theme = theme();
    let policy = ImagePolicy::default();
    let renderer = Renderer {
        card: &card,
        state: &state,
        theme: &theme,
        policy: &policy,
        budget: Budget::new(cap),
        key: SharedString::from("test-row"),
        on_event: Rc::new(|_, _, _| {}),
    };
    let element = renderer.render();
    drop(element);
    let used = renderer.budget.used();
    (renderer.budget, used)
}

/// The runaway template goes through the real renderer and lands on the
/// budget panel: the cap is spent, nothing panics, no stack blows.
#[test]
fn self_referencing_template_renders_the_budget_panel() {
    let (budget, used) = render_with(
        json!({
            "components": [
                {"id": "root", "component": "Column", "children": {"componentId": "root", "path": "/items"}}
            ],
            "data": {"items": [1, 2, 3, 4, 5]}
        }),
        300,
    );
    assert!(budget.exceeded(), "used {used}");
    assert_eq!(used, 300);
}

/// Every fixture renders inside the default budget, and the six mockup
/// cards are far below it.
#[test]
fn fixtures_render_within_budget() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let mut rendered = 0;
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|x| x != "json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap().replace("fixture://", "file:///nowhere/");
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let (budget, used) = render_with(json, crate::budget::MAX_NODES);
        assert!(!budget.exceeded(), "{}: used {used}", path.display());
        assert!(used < 200, "{}: used {used}", path.display());
        rendered += 1;
    }
    eprintln!("fixtures asked=6 rendered={rendered}");
    assert_eq!(rendered, 6);
}

/// Unknown components, missing children, a tab array past the cap, and a
/// hostile image all render to boxes, never a panic.
#[test]
fn broken_trees_render_to_fallback_boxes() {
    let many_tabs: Vec<_> = (0..40).map(|i| json!({"title": format!("t{i}"), "child": "u"})).collect();
    let (budget, used) = render_with(
        json!({"components": [
            {"id": "root", "component": "Column", "children": ["u", "missing", "tabs", "img", "btn"]},
            {"id": "u", "component": "Slider", "min": 0},
            {"id": "tabs", "component": "Tabs", "tabs": many_tabs},
            {"id": "img", "component": "Image", "url": "https://evil.example/x.png?t=1"},
            {"id": "btn", "component": "Button", "child": "missing"}
        ]}),
        crate::budget::MAX_NODES,
    );
    assert!(!budget.exceeded());
    // Every visited id takes a unit, missing ones included (their box is
    // an element too): root, u, missing, tabs, 16 headers, u (tab body),
    // img, btn, btn's missing child = 24.
    assert_eq!(used, 24);
}
