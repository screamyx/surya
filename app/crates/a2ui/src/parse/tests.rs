//! Parser tests: the accepted shapes, and the promise that broken input
//! degrades to diagnostics instead of a panic.

use serde_json::{Value, json};

use crate::model::*;
use crate::parse::{parse_card, parse_card_str};

fn text_card() -> Value {
    json!([
        {"version": "v0.9.1", "createSurface": {"surfaceId": "s1", "catalogId": "cat"}},
        {"version": "v0.9.1", "updateComponents": {"surfaceId": "s1", "components": [
            {"id": "root", "component": "Column", "children": ["t", "b"]},
            {"id": "t", "component": "Text", "text": {"path": "/name"}, "variant": "h2"},
            {"id": "b", "component": "Button", "child": "t", "variant": "primary",
             "action": {"event": {"name": "go", "context": {"n": {"path": "/name"}}}}}
        ]}},
        {"version": "v0.9.1", "updateDataModel": {"surfaceId": "s1", "path": "/name", "value": "Ann"}}
    ])
}

#[test]
fn envelope_list_parses_surface_components_and_data() {
    let card = parse_card(&text_card());
    assert_eq!(card.id, "s1");
    assert_eq!(card.catalog_id, "cat");
    assert!(card.errors.is_empty(), "{:?}", card.errors);
    assert_eq!(card.component_ids(), vec!["b", "root", "t"]);
    assert_eq!(card.data, json!({"name": "Ann"}));
    assert!(matches!(&card.get("t").unwrap().kind, ComponentKind::Text { variant: TextVariant::H2, text: Dynamic::Path(p) } if p == "/name"));
    assert!(matches!(&card.get("b").unwrap().kind, ComponentKind::Button { variant: ButtonVariant::Primary, action: Action::Event(e), .. } if e.name == "go"));
}

#[test]
fn jsonl_and_string_inputs_parse_the_same() {
    let lines: Vec<String> = text_card()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.to_string())
        .collect();
    let from_jsonl = parse_card_str(&lines.join("\n"));
    let from_string = parse_card(&Value::String(text_card().to_string()));
    assert_eq!(from_jsonl, parse_card(&text_card()));
    assert_eq!(from_string, parse_card(&text_card()));
}

#[test]
fn shorthand_and_wrappers_parse() {
    let card = parse_card(&json!({"surfaceId": "x", "components": [{"id": "root", "component": "Text", "text": "hi"}], "data": {"k": 1}}));
    assert_eq!((card.id.as_str(), card.errors.len()), ("x", 0));
    assert_eq!(card.data["k"], 1);
    let wrapped = parse_card(&json!({"messages": [{"updateComponents": {"surfaceId": "y", "components": [{"id": "root", "component": "Text", "text": "hi"}]}}]}));
    assert_eq!((wrapped.id.as_str(), wrapped.errors.len()), ("y", 0));
}

#[test]
fn broken_input_never_panics_and_reports() {
    assert!(parse_card(&json!(42)).errors.iter().any(|e| e.contains("object or array")));
    assert!(parse_card(&json!({"components": []})).errors.contains(&"no components".to_string()));
    let no_root = parse_card(&json!({"components": [{"id": "a", "component": "Text", "text": "x"}]}));
    assert!(no_root.root().is_none());
    assert!(no_root.errors.iter().any(|e| e.contains("root")));
    let bad_items = parse_card(&json!({"components": [7, {"component": "Text"}, {"id": "root"}, {"id": "root", "component": "Text", "text": "ok"}]}));
    assert_eq!(bad_items.errors.len(), 3, "{:?}", bad_items.errors);
    assert!(bad_items.root().is_some());
    let text = parse_card_str("{not json\n{\"updateComponents\":{\"surfaceId\":\"z\",\"components\":[{\"id\":\"root\",\"component\":\"Text\",\"text\":\"t\"}]}}");
    assert!(text.errors.iter().any(|e| e.starts_with("line 1")));
    assert!(text.root().is_some());
}

#[test]
fn unknown_and_partial_components_degrade_not_fail() {
    let card = parse_card(&json!({"components": [
        {"id": "root", "component": "Column", "children": {"componentId": "row", "path": "/items"}, "align": "bogus"},
        {"id": "row", "component": "Slider", "min": 0, "max": 10},
        {"id": "img", "component": "Image"},
        {"id": "tabs", "component": "Tabs", "tabs": [{"title": "A"}, {"title": "B", "child": "img"}]},
        {"id": "btn", "component": "Button", "child": "img", "weight": 2}
    ]}));
    assert!(card.errors.is_empty(), "{:?}", card.errors);
    assert!(matches!(&card.get("root").unwrap().kind, ComponentKind::Column { children: ChildList::Template { path, .. }, align: Align::Stretch, .. } if path == "/items"));
    assert!(matches!(&card.get("row").unwrap().kind, ComponentKind::Unknown { name, .. } if name == "Slider"));
    assert!(matches!(&card.get("img").unwrap().kind, ComponentKind::Image { url: Dynamic::Literal(u), .. } if u.is_empty()));
    assert!(matches!(&card.get("tabs").unwrap().kind, ComponentKind::Tabs { tabs } if tabs.len() == 1 && tabs[0].child == "img"));
    let btn = card.get("btn").unwrap();
    assert_eq!(btn.weight, Some(2.0));
    assert!(matches!(&btn.kind, ComponentKind::Button { action: Action::None, .. }));
}
