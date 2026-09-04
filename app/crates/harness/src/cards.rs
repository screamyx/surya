//! The A2UI envelope list a `show_card` call carries, normalized once for
//! every harness: a model may send the full envelope stream, one envelope,
//! or the `{surfaceId, components, data}` shorthand. The renderer parses
//! any of them, but the wire event ([`zeron_proto::AgentEvent::Card`])
//! carries the list form so every consumer sees one shape.

use serde_json::{Value, json};

/// The basic catalog's own declared id (the catalog FILE says `v0_9`).
pub const BASIC_CATALOG_ID: &str =
    "https://a2ui.org/specification/v0_9/catalogs/basic/catalog.json";

/// The tool names whose call IS a card (surya decision 14: agents draw
/// cards as `show_card` tool calls, never as reply text). Bare or
/// MCP-prefixed (`mcp__surya__show_card`, or any server's `__show_card`).
pub fn is_card_tool(name: &str) -> bool {
    name == "show_card" || name.ends_with("__show_card")
}

/// `(surface_id, envelopes)` for a card payload in any accepted shape.
/// `default_surface` names the surface when the payload does not.
pub fn envelope_list(input: &Value, default_surface: &str) -> (String, Vec<Value>) {
    let input = input
        .get("card")
        .or_else(|| input.get("json"))
        .unwrap_or(input);
    let input = match input {
        Value::String(text) => serde_json::from_str::<Value>(text).unwrap_or(Value::Null),
        other => other.clone(),
    };
    let envelopes: Vec<Value> = match &input {
        Value::Array(items) => items.clone(),
        Value::Object(obj) if obj.get("messages").is_some_and(Value::is_array) => obj["messages"]
            .as_array()
            .cloned()
            .unwrap_or_default(),
        Value::Object(obj)
            if ["createSurface", "updateComponents", "updateDataModel"]
                .iter()
                .any(|k| obj.contains_key(*k)) =>
        {
            vec![input.clone()]
        }
        Value::Object(obj) => {
            let surface = obj
                .get("surfaceId")
                .or_else(|| obj.get("id"))
                .and_then(Value::as_str)
                .unwrap_or(default_surface);
            let catalog = obj
                .get("catalogId")
                .and_then(Value::as_str)
                .unwrap_or(BASIC_CATALOG_ID);
            let mut out = vec![
                json!({"version": "v0.9.1", "createSurface": {"surfaceId": surface, "catalogId": catalog}}),
                json!({"version": "v0.9.1", "updateComponents": {
                    "surfaceId": surface,
                    "components": obj.get("components").cloned().unwrap_or(Value::Array(Vec::new()))
                }}),
            ];
            if let Some(data) = obj.get("data").or_else(|| obj.get("dataModel")) {
                out.push(json!({"version": "v0.9.1", "updateDataModel": {"surfaceId": surface, "value": data}}));
            }
            out
        }
        _ => Vec::new(),
    };
    let surface_id = envelopes
        .iter()
        .find_map(|e| {
            e.get("createSurface")
                .or_else(|| e.get("updateComponents"))
                .and_then(|m| m.get("surfaceId"))
                .and_then(Value::as_str)
        })
        .unwrap_or(default_surface)
        .to_owned();
    (surface_id, envelopes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shorthand_expands_to_three_envelopes() {
        let (surface, list) = envelope_list(
            &json!({"surfaceId": "s1", "components": [{"id": "root", "component": "Text", "text": "hi"}], "data": {"a": 1}}),
            "fallback",
        );
        assert_eq!(surface, "s1");
        assert_eq!(list.len(), 3);
        assert_eq!(list[0]["createSurface"]["catalogId"], BASIC_CATALOG_ID);
        assert_eq!(list[1]["updateComponents"]["components"][0]["id"], "root");
        assert_eq!(list[2]["updateDataModel"]["value"]["a"], 1);
    }

    #[test]
    fn lists_and_wrappers_pass_through() {
        let list = json!([{"createSurface": {"surfaceId": "x", "catalogId": "c"}}, {"updateComponents": {"surfaceId": "x", "components": []}}]);
        assert_eq!(envelope_list(&list, "d"), ("x".into(), list.as_array().cloned().unwrap()));
        assert_eq!(envelope_list(&json!({"card": list.clone()}), "d").0, "x");
        assert_eq!(envelope_list(&json!({"messages": list.clone()}), "d").1.len(), 2);
        assert_eq!(envelope_list(&Value::String(list.to_string()), "d").0, "x");
        let (surface, single) = envelope_list(&json!({"updateComponents": {"surfaceId": "y", "components": []}}), "d");
        assert_eq!((surface.as_str(), single.len()), ("y", 1));
        assert_eq!(envelope_list(&json!(42), "d"), ("d".into(), Vec::new()));
        assert!(!is_card_tool("mcp__surya__open_file"));
        assert!(is_card_tool("mcp__surya__show_card"));
    }
}
