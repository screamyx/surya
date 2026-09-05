//! Browser tools for the `surya-mcp` server: `browser_open`,
//! `browser_snapshot`, `browser_click`, `browser_type`, `browser_screenshot`
//! and `browser_eval`. Each is one `Browser.Call` engine RPC over the same
//! IPC socket the task tools dial; the engine hands the op to the app's
//! browser pane and returns what the page said (`zeron_engine::browser_rpc`).
//! So the agent acts on the page the owner is looking at, in the app.
//!
//! `browser_screenshot` comes back twice: as an MCP image content block the
//! model can see, and as a card in the card store (like `show_card`) so the
//! transcript shows the picture where the tool ran.
//!
//! Every call carries the engine's pane token: `ZERON_IPC_TOKEN`, else the
//! `ipc-token` file under `ZERON_DATA_DIR`, else under the engine's own
//! default data dir (`~/.zeron`, then `~/.surya` after decision 23's
//! rename); a 0600 file of the engine's user, see [`pane_token`]. The
//! engine refuses Browser.* without it, so a stranger's process on the box
//! cannot drive the owner's pane through this server either.

use serde_json::{Value, json};
use zeron_rpc::methods;

use crate::cards;
use crate::config::Config;
use crate::shapes::{DEFAULT_CATALOG_ID, PROTOCOL_VERSION};
use crate::tasks::engine_target;

pub const OPEN: &str = "browser_open";
pub const SNAPSHOT: &str = "browser_snapshot";
pub const CLICK: &str = "browser_click";
pub const TYPE: &str = "browser_type";
pub const SCREENSHOT: &str = "browser_screenshot";
pub const EVAL: &str = "browser_eval";

/// The largest screenshot that goes into a card as a `data:` URI; the
/// renderer's own cap (`zeron_a2ui::images::MAX_DATA_URI_BYTES`).
const MAX_CARD_IMAGE_BYTES: usize = 2 * 1024 * 1024;

pub fn handles(name: &str) -> bool {
    matches!(name, OPEN | SNAPSHOT | CLICK | TYPE | SCREENSHOT | EVAL)
}

/// MCP `tools/list` entries.
pub fn tool_specs() -> Vec<Value> {
    let id = json!({
        "type": "integer",
        "description": "An element id from the latest browser_snapshot ([n] at the start of its line)."
    });
    vec![
        json!({
            "name": OPEN,
            "description": "Open a URL in the app's browser pane, the page the user is looking at, and wait for it to load. http(s) only.",
            "inputSchema": {
                "type": "object",
                "properties": { "url": { "type": "string" } },
                "required": ["url"],
            },
        }),
        json!({
            "name": SNAPSHOT,
            "description": "Read the page in the browser pane: one line per heading, text block and control, in order. Controls carry an id like [3] for browser_click and browser_type. Call it again after anything that changes the page; ids are not stable across changes.",
            "inputSchema": { "type": "object", "properties": {} },
        }),
        json!({
            "name": CLICK,
            "description": "Click an element in the browser pane by its snapshot id, with a real mouse click at its centre.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": id },
                "required": ["id"],
            },
        }),
        json!({
            "name": TYPE,
            "description": "Type text into a field in the browser pane by its snapshot id, replacing what it holds.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": id, "text": { "type": "string" } },
                "required": ["id", "text"],
            },
        }),
        json!({
            "name": SCREENSHOT,
            "description": "A PNG of the page in the browser pane, as the user sees it. Returned as an image, and shown in the transcript as a card.",
            "inputSchema": { "type": "object", "properties": {} },
        }),
        json!({
            "name": EVAL,
            "description": "Run a JavaScript expression in the page and return its value (JSON). Promises are awaited.",
            "inputSchema": {
                "type": "object",
                "properties": { "js": { "type": "string" } },
                "required": ["js"],
            },
        }),
    ]
}

/// The pane token: `ZERON_IPC_TOKEN`, else the engine's `ipc-token` file
/// under `ZERON_DATA_DIR`, else the engine's own default data dir.
///
/// The last step used to be `~/.surya`, which is this crate's CARD workspace
/// (`config::home_dir`), not the engine's data dir - the engine defaults to
/// `~/.zeron` (`apps/zeron/src/main.rs`). On a default install that meant no
/// token was ever found and every browser tool errored.
///
/// Both names are tried because decision 23's rename from `.zeron` to
/// `.surya` is in flight: `zeron_engine::data_dir::adopt_and_report` copies
/// the dir, so during the changeover either can be the live one. `.zeron`
/// goes first, because that is what the app writes today.
pub fn pane_token() -> Option<String> {
    resolve_pane_token(
        std::env::var("ZERON_IPC_TOKEN").ok(),
        std::env::var_os("ZERON_DATA_DIR")
            .filter(|d| !d.is_empty())
            .map(std::path::PathBuf::from),
        std::env::var_os("HOME").map(std::path::PathBuf::from),
    )
}

/// The resolution itself, with the environment passed in so it is testable
/// without setting process-global variables from three tests at once.
pub fn resolve_pane_token(
    env_token: Option<String>,
    data_dir: Option<std::path::PathBuf>,
    home: Option<std::path::PathBuf>,
) -> Option<String> {
    let non_empty = |t: String| {
        let t = t.trim().to_string();
        (!t.is_empty()).then_some(t)
    };
    if let Some(t) = env_token.and_then(non_empty) {
        return Some(t);
    }
    let read = |dir: std::path::PathBuf| {
        std::fs::read_to_string(dir.join("ipc-token"))
            .ok()
            .and_then(non_empty)
    };
    if let Some(dir) = data_dir
        && let Some(t) = read(dir)
    {
        return Some(t);
    }
    let home = home?;
    read(home.join(".zeron")).or_else(|| read(home.join(".surya")))
}

#[cfg(test)]
mod token_tests {
    use super::resolve_pane_token;

    fn write(dir: &std::path::Path, token: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("ipc-token"), token).unwrap();
    }

    /// Case 1, the env is set: it wins, and no file is read.
    #[test]
    fn the_env_token_wins() {
        let home = tempfile::tempdir().unwrap();
        write(&home.path().join(".zeron"), "from-file");
        assert_eq!(
            resolve_pane_token(
                Some("  from-env  ".into()),
                None,
                Some(home.path().to_path_buf())
            )
            .as_deref(),
            Some("from-env"),
            "trimmed, and the file is not consulted"
        );
    }

    /// Case 2, a default install: no env, no ZERON_DATA_DIR, so the engine's
    /// own default dir is where the token is. This is the case that used to
    /// fail - the old code looked in `~/.surya`, the card workspace.
    #[test]
    fn the_default_install_finds_the_engines_own_dir() {
        let home = tempfile::tempdir().unwrap();
        write(&home.path().join(".zeron"), "engine-token");
        assert_eq!(
            resolve_pane_token(None, None, Some(home.path().to_path_buf())).as_deref(),
            Some("engine-token")
        );

        // After decision 23's rename the dir is `.surya`; both are tried, so
        // the changeover does not break the tools either way.
        let renamed = tempfile::tempdir().unwrap();
        write(&renamed.path().join(".surya"), "renamed-token");
        assert_eq!(
            resolve_pane_token(None, None, Some(renamed.path().to_path_buf())).as_deref(),
            Some("renamed-token")
        );
    }

    /// `ZERON_DATA_DIR` beats the home default, and an empty file is not a
    /// token: falling through to the default dir beats sending "".
    #[test]
    fn an_explicit_data_dir_wins_and_an_empty_file_does_not_count() {
        let home = tempfile::tempdir().unwrap();
        write(&home.path().join(".zeron"), "home-token");
        let explicit = tempfile::tempdir().unwrap();
        write(explicit.path(), "explicit-token");
        assert_eq!(
            resolve_pane_token(
                None,
                Some(explicit.path().to_path_buf()),
                Some(home.path().to_path_buf())
            )
            .as_deref(),
            Some("explicit-token")
        );

        let blank = tempfile::tempdir().unwrap();
        write(blank.path(), "   \n");
        assert_eq!(
            resolve_pane_token(
                None,
                Some(blank.path().to_path_buf()),
                Some(home.path().to_path_buf())
            )
            .as_deref(),
            Some("home-token"),
            "a blank file falls through instead of authenticating with nothing"
        );
    }
}

/// One tool call from the synchronous server loop. Returns the MCP result
/// object (`content` + `isError`), or an error string the server wraps.
pub fn call_blocking(
    config: &Config,
    name: &str,
    args: Value,
    tool_use_id: &str,
) -> Result<Value, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("tokio runtime: {e}"))?;
    let token = pane_token().ok_or(
        "no pane token: set ZERON_IPC_TOKEN or run under the engine's ZERON_DATA_DIR (its ipc-token file)",
    )?;
    let answer = runtime.block_on(async {
        let target = engine_target();
        let client = zeron_rpc::connect_ws_with_token(&target.url, target.token.as_deref())
            .await
            .map_err(|e| format!("no surya engine at {}: {e}", target.url))?;
        client
            .call(methods::BROWSER_CALL, json!({ "op": name, "args": args, "token": token }))
            .await
            .map_err(|e| e.to_string())
    })?;
    Ok(shape_result(config, name, answer, tool_use_id))
}

/// Turn the pane's answer into what the model reads.
fn shape_result(config: &Config, name: &str, answer: Value, tool_use_id: &str) -> Value {
    match name {
        SNAPSHOT => text(answer["snapshot"].as_str().unwrap_or("").to_string()),
        SCREENSHOT => screenshot_result(config, &answer, tool_use_id),
        _ => text(answer.to_string()),
    }
}

fn text(text: String) -> Value {
    json!({ "content": [{ "type": "text", "text": text }], "isError": false })
}

/// The image for the model, and a card for the transcript when it fits.
fn screenshot_result(config: &Config, answer: &Value, tool_use_id: &str) -> Value {
    let Some(png) = answer["png_base64"].as_str() else {
        return json!({
            "content": [{ "type": "text", "text": "the pane returned no image" }],
            "isError": true,
        });
    };
    let note = if png.len() <= MAX_CARD_IMAGE_BYTES {
        let card = screenshot_card(png);
        match cards::show(config, &json!({ "card": card }), tool_use_id) {
            Ok(shown) => format!("screenshot shown in the transcript as card {}", shown.card_id),
            Err(e) => format!("screenshot taken; the transcript card could not be recorded: {e}"),
        }
    } else {
        "screenshot taken; too large for a transcript card".to_string()
    };
    json!({
        "content": [
            { "type": "image", "data": png, "mimeType": "image/png" },
            { "type": "text", "text": note },
        ],
        "isError": false,
    })
}

/// A card that is the picture, uncropped: `fit: contain` on a large
/// feature image. The record shape's `cover` would crop a page.
fn screenshot_card(png_base64: &str) -> Value {
    let catalog = std::env::var("SURYA_CATALOG_ID")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_CATALOG_ID.into());
    let surface_id = format!("shot_{}", uuid::Uuid::new_v4().simple());
    json!([
        {
            "version": PROTOCOL_VERSION,
            "createSurface": { "surfaceId": surface_id, "catalogId": catalog, "sendDataModel": true }
        },
        {
            "version": PROTOCOL_VERSION,
            "updateComponents": {
                "surfaceId": surface_id,
                "components": [
                    { "id": "title", "component": "Text", "text": "Browser screenshot", "variant": "caption" },
                    {
                        "id": "shot",
                        "component": "Image",
                        "url": format!("data:image/png;base64,{png_base64}"),
                        "fit": "contain",
                        "variant": "largeFeature",
                        "description": "The page in the browser pane"
                    },
                    { "id": "body", "component": "Column", "children": ["title", "shot"], "align": "stretch" },
                    { "id": "root", "component": "Card", "child": "body" }
                ]
            }
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_is_advertised_with_a_schema_and_handled() {
        let specs = tool_specs();
        assert_eq!(specs.len(), 6);
        for spec in &specs {
            let name = spec["name"].as_str().unwrap();
            assert!(handles(name), "{name}");
            assert_eq!(spec["inputSchema"]["type"], "object");
        }
        assert!(!handles("show_card"));
    }

    #[test]
    fn a_snapshot_comes_back_as_plain_text() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            agent_id: "a".into(),
            workspace: "w".into(),
            workspace_root: dir.path().into(),
            card_store: dir.path().join("cards.jsonl"),
            mail_socket: dir.path().join("mail.sock"),
            mail_log: dir.path().join("mail.jsonl"),
        };
        let out = shape_result(&config, SNAPSHOT, json!({ "snapshot": "[1] link \"Home\"" }), "t1");
        assert_eq!(out["content"][0]["text"], "[1] link \"Home\"");
        assert_eq!(out["isError"], false);
    }

    #[test]
    fn a_screenshot_is_an_image_block_and_a_card_in_the_store() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            agent_id: "a".into(),
            workspace: "w".into(),
            workspace_root: dir.path().into(),
            card_store: dir.path().join("cards.jsonl"),
            mail_socket: dir.path().join("mail.sock"),
            mail_log: dir.path().join("mail.jsonl"),
        };
        let out = shape_result(&config, SCREENSHOT, json!({ "png_base64": "iVBORw0KGgo=" }), "toolu_9");
        assert_eq!(out["content"][0]["type"], "image");
        assert_eq!(out["content"][0]["mimeType"], "image/png");
        assert_eq!(out["content"][0]["data"], "iVBORw0KGgo=");
        assert!(out["content"][1]["text"].as_str().unwrap().contains("card card_"));
        let store = std::fs::read_to_string(config.card_store).unwrap();
        let record: Value = serde_json::from_str(store.lines().next().unwrap()).unwrap();
        assert_eq!(record["tool_use_id"], "toolu_9");
        let components = &record["a2ui"][1]["updateComponents"]["components"];
        let image = components.as_array().unwrap().iter().find(|c| c["component"] == "Image").unwrap();
        assert!(image["url"].as_str().unwrap().starts_with("data:image/png;base64,"));
        assert_eq!(image["fit"], "contain");
        assert!(components.as_array().unwrap().iter().any(|c| c["id"] == "root"));
    }

    #[test]
    fn a_pane_without_an_image_is_a_readable_error() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            agent_id: "a".into(),
            workspace: "w".into(),
            workspace_root: dir.path().into(),
            card_store: dir.path().join("cards.jsonl"),
            mail_socket: dir.path().join("mail.sock"),
            mail_log: dir.path().join("mail.jsonl"),
        };
        let out = shape_result(&config, SCREENSHOT, json!({}), "t");
        assert_eq!(out["isError"], true);
    }
}
