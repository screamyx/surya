//! `show_card` and `list_cards`.
//!
//! `show_card` takes either a short-form card (see [`crate::shapes`]) or raw
//! A2UI messages, normalizes both to one A2UI envelope stream, appends that
//! stream to the card store, and hands the agent back a `card_id`. The agent's
//! transcript therefore carries a small result; the app reads the card body
//! out of the store by id.

use serde_json::{Value, json};

use crate::config::{Config, append_line};
use crate::shapes::{self, BUILT_IN_SHAPES, DEFAULT_CATALOG_ID, PROTOCOL_VERSION};

/// The catalog the surface declares. Overridable so the native renderer can
/// swap in surya's own catalog without a change here.
fn catalog_id() -> String {
    std::env::var("SURYA_CATALOG_ID")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_CATALOG_ID.into())
}

/// One shown card, as the store records it and the app replays it.
#[derive(Debug)]
pub struct Card {
    pub card_id: String,
    pub surface_id: String,
    pub messages: Vec<Value>,
}

/// Turn the tool's `card` argument into an A2UI envelope stream.
///
/// Accepted, in order: a list of A2UI envelope messages, a single envelope
/// message, or a short-form card carrying `shape`.
pub fn normalize(card: &Value, surface_id: &str) -> Result<Vec<Value>, String> {
    if let Some(messages) = card.as_array() {
        if messages.is_empty() {
            return Err("card is an empty list; send A2UI messages or a short-form card".into());
        }
        return Ok(messages.to_vec());
    }
    if !card.is_object() {
        return Err("card must be an object or a list of A2UI messages".into());
    }
    if is_envelope(card) {
        return Ok(vec![card.clone()]);
    }
    let (components, data) = shapes::expand(card)?;
    let mut messages = vec![
        json!({
            "version": PROTOCOL_VERSION,
            "createSurface": {
                "surfaceId": surface_id,
                "catalogId": catalog_id(),
                "sendDataModel": true,
            }
        }),
        json!({
            "version": PROTOCOL_VERSION,
            "updateComponents": { "surfaceId": surface_id, "components": components }
        }),
    ];
    if data.as_object().is_some_and(|d| !d.is_empty()) {
        messages.push(json!({
            "version": PROTOCOL_VERSION,
            "updateDataModel": { "surfaceId": surface_id, "path": "/", "value": data }
        }));
    }
    Ok(messages)
}

const ENVELOPE_KEYS: &[&str] = &[
    "createSurface",
    "updateComponents",
    "updateDataModel",
    "deleteSurface",
];

fn is_envelope(card: &Value) -> bool {
    ENVELOPE_KEYS.iter().any(|key| card.get(key).is_some())
}

/// Handle one `show_card` call.
pub fn show(config: &Config, arguments: &Value) -> Result<Card, String> {
    let card = arguments
        .get("card")
        .ok_or("show_card needs a \"card\" argument")?;
    let card_id = format!("card_{}", uuid::Uuid::new_v4().simple());
    let surface_id = arguments
        .get("surface_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| card_id.clone());
    let messages = normalize(card, &surface_id)?;
    let record = json!({
        "card_id": card_id,
        "surface_id": surface_id,
        "agent_id": config.agent_id,
        "workspace": config.workspace,
        "at": chrono::Utc::now().to_rfc3339(),
        "a2ui": messages,
    });
    append_line(&config.card_store, &record.to_string())
        .map_err(|e| format!("could not write the card store at {:?}: {e}", config.card_store))?;
    Ok(Card {
        card_id,
        surface_id,
        messages,
    })
}

/// A card this workspace brings with it (decision 14: data, never code).
pub struct WorkspaceCard {
    pub name: String,
    pub description: String,
    pub file: String,
}

/// Read `.surya/cards/*.json` under the workspace root. A folder that is
/// missing is normal, not an error: most workspaces add no cards of their own.
pub fn workspace_cards(config: &Config) -> Vec<WorkspaceCard> {
    let dir = config.cards_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut cards: Vec<WorkspaceCard> = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter_map(|entry| {
            let path = entry.path();
            let stem = path.file_stem()?.to_string_lossy().to_string();
            let body: Value = std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or(Value::Null);
            let name = body
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&stem)
                .to_string();
            let description = body
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("(no description in the card file)")
                .to_string();
            Some(WorkspaceCard {
                name,
                description,
                file: path.to_string_lossy().to_string(),
            })
        })
        .collect();
    cards.sort_by(|a, b| a.name.cmp(&b.name));
    cards
}

/// Handle one `list_cards` call: the built-in shapes plus this workspace's own.
pub fn list(config: &Config) -> Value {
    let built_in: Vec<Value> = BUILT_IN_SHAPES
        .iter()
        .map(|(name, description)| json!({ "shape": name, "description": description }))
        .collect();
    let workspace: Vec<Value> = workspace_cards(config)
        .into_iter()
        .map(|card| {
            json!({ "name": card.name, "description": card.description, "file": card.file })
        })
        .collect();
    json!({
        "built_in": built_in,
        "workspace": workspace,
        "workspace_cards_dir": config.cards_dir().to_string_lossy(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn config(root: PathBuf, store: PathBuf) -> Config {
        Config {
            agent_id: "seat-1".into(),
            workspace: "demo".into(),
            workspace_root: root,
            card_store: store,
            mail_socket: PathBuf::from("/nonexistent/mail.sock"),
            mail_log: PathBuf::from("/nonexistent/mail.jsonl"),
        }
    }

    #[test]
    fn a_short_form_card_becomes_a_create_plus_update_stream() {
        let messages = normalize(&json!({"shape":"metric","title":"T","value":"1"}), "s1").unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["createSurface"]["surfaceId"], "s1");
        assert_eq!(messages[0]["version"], PROTOCOL_VERSION);
        assert_eq!(messages[1]["updateComponents"]["surfaceId"], "s1");
    }

    #[test]
    fn raw_a2ui_passes_through_untouched() {
        let raw = json!([{"version":"v0.9.1","createSurface":{"surfaceId":"x","catalogId":"c"}}]);
        let messages = normalize(&raw, "ignored").unwrap();
        assert_eq!(messages, raw.as_array().unwrap().clone());

        let single = json!({"version":"v0.9.1","deleteSurface":{"surfaceId":"x"}});
        assert_eq!(normalize(&single, "ignored").unwrap(), vec![single]);
    }

    #[test]
    fn show_writes_one_record_per_call() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("cards.jsonl");
        let config = config(dir.path().to_path_buf(), store.clone());
        let asked = 2;
        for _ in 0..asked {
            show(&config, &json!({"card": {"shape": "table", "columns": ["A"], "rows": [["1"]]}}))
                .unwrap();
        }
        let written = std::fs::read_to_string(&store).unwrap().lines().count();
        assert_eq!((asked, written), (2, 2), "asked={asked} written={written}");
    }

    #[test]
    fn show_rejects_a_card_with_no_shape_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("cards.jsonl");
        let config = config(dir.path().to_path_buf(), store.clone());
        let error = show(&config, &json!({"card": {"title": "no shape"}})).unwrap_err();
        assert!(error.contains("shape"), "{error}");
        assert!(!store.exists(), "a rejected card writes nothing");
    }

    #[test]
    fn list_cards_reads_the_workspace_catalog() {
        let dir = tempfile::tempdir().unwrap();
        let cards = dir.path().join(".surya").join("cards");
        std::fs::create_dir_all(&cards).unwrap();
        std::fs::write(
            cards.join("release.json"),
            r#"{"name":"release","description":"One release and its checks","shape":"record"}"#,
        )
        .unwrap();
        std::fs::write(cards.join("notes.txt"), "ignored").unwrap();
        let config = config(dir.path().to_path_buf(), dir.path().join("cards.jsonl"));
        let listed = list(&config);
        assert_eq!(listed["built_in"].as_array().unwrap().len(), 6);
        let workspace = listed["workspace"].as_array().unwrap();
        assert_eq!(workspace.len(), 1, "only .json files count");
        assert_eq!(workspace[0]["name"], "release");
        assert_eq!(workspace[0]["description"], "One release and its checks");
    }

    #[test]
    fn a_workspace_with_no_cards_folder_lists_only_the_built_ins() {
        let dir = tempfile::tempdir().unwrap();
        let config = config(dir.path().to_path_buf(), dir.path().join("cards.jsonl"));
        let listed = list(&config);
        assert_eq!(listed["workspace"].as_array().unwrap().len(), 0);
        assert_eq!(listed["built_in"].as_array().unwrap().len(), 6);
    }
}
