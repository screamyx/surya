//! What an agent can do to the page: open a URL, read it, click, type, look,
//! and run its own script. Every op is one or two DevTools calls through
//! [`crate::devtools`], on the same browser the owner is looking at.
//!
//! The ops are named by the MCP tools that call them (`browser_open` and so
//! on, surya-mcp `browser.rs`); [`run`] is the one entry point, so the pane
//! side dispatches by op name and never knows the shapes. The result of each
//! op is a JSON object the tool hands back to the agent unchanged.
//!
//! Loads are awaited through `client.rs`'s `on_load_end`, not polled: a
//! [`wait_for_load`] future resolves when the main frame finishes. There is
//! no timer in this crate; the caller puts a deadline around `run`.

use std::sync::Mutex;

use futures::channel::oneshot;
use serde_json::{Value, json};

use crate::devtools;

/// The script that reads the page; see the file for the line format.
const SNAPSHOT_JS: &str = include_str!("snapshot.js");

static LOAD_WAITERS: Mutex<Vec<oneshot::Sender<()>>> = Mutex::new(Vec::new());

/// Resolves on the next main-frame load end. Take it *before* starting the
/// navigation, or the end may already have gone by.
pub fn wait_for_load() -> impl std::future::Future<Output = ()> {
    let (tx, rx) = oneshot::channel();
    if let Ok(mut w) = LOAD_WAITERS.lock() {
        w.push(tx);
    }
    async move {
        let _ = rx.await;
    }
}

/// The main frame finished loading (success or error page). From
/// `client.rs`.
pub(crate) fn on_load_end() {
    let waiters: Vec<_> = LOAD_WAITERS.lock().map(|mut w| w.drain(..).collect()).unwrap_or_default();
    for w in waiters {
        let _ = w.send(());
    }
}

/// Run one op by name. Unknown ops and bad arguments are errors the agent
/// can read and fix.
pub async fn run(op: &str, args: &Value) -> Result<Value, String> {
    if crate::disabled() {
        return Err("the browser pane is disabled in this app (SURYA_NO_BROWSER)".into());
    }
    match op {
        "browser_open" => open(str_arg(args, "url")?).await,
        "browser_snapshot" => snapshot().await,
        "browser_click" => click(id_arg(args)?).await,
        "browser_type" => type_text(id_arg(args)?, str_arg(args, "text")?).await,
        "browser_screenshot" => screenshot().await,
        "browser_eval" => eval(str_arg(args, "js")?).await,
        other => Err(format!("unknown browser op {other:?}")),
    }
}

fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("missing argument {key:?}"))
}

fn id_arg(args: &Value) -> Result<u64, String> {
    match args.get("id") {
        Some(Value::Number(n)) => n.as_u64().ok_or_else(|| "id must be a positive integer".into()),
        Some(Value::String(s)) => s.trim().parse().map_err(|_| format!("id {s:?} is not a number")),
        _ => Err("missing argument \"id\" (from browser_snapshot)".into()),
    }
}

/// Load a URL and wait for it. The same allowlist as the address bar: a
/// `file:` URL or a private scheme is refused, not loaded.
async fn open(url: &str) -> Result<Value, String> {
    let resolved = crate::page::navigate_to(url);
    if resolved.is_empty() {
        return Err(format!("refused to open {url:?}: only http(s) addresses and localhost are allowed"));
    }
    crate::reopen();
    let loaded = wait_for_load();
    crate::navigate(&resolved);
    loaded.await;
    let page = crate::page::page();
    Ok(json!({
        "url": page.url,
        "title": page.title,
        "error": page.error,
    }))
}

/// Evaluate an expression and return its value (or the thrown error).
async fn evaluate(expression: &str) -> Result<Value, String> {
    let result = devtools::call(
        "Runtime.evaluate",
        json!({
            "expression": expression,
            "returnByValue": true,
            "awaitPromise": true,
        }),
    )
    .await?;
    if let Some(details) = result.get("exceptionDetails") {
        let text = details["exception"]["description"]
            .as_str()
            .or_else(|| details["text"].as_str())
            .unwrap_or("script threw");
        return Err(format!("page script failed: {text}"));
    }
    Ok(result["result"]["value"].clone())
}

async fn snapshot() -> Result<Value, String> {
    let value = evaluate(SNAPSHOT_JS).await?;
    let text = value.as_str().ok_or("the snapshot script returned no text")?;
    Ok(json!({ "snapshot": text }))
}

/// Where an element's centre is in CSS pixels, after scrolling it into view.
async fn locate(id: u64) -> Result<(f64, f64), String> {
    let js = format!(
        "(() => {{ const el = document.querySelector('[data-surya-id=\"{id}\"]'); \
         if (!el) return null; el.scrollIntoView({{ block: 'center', inline: 'center' }}); \
         const r = el.getBoundingClientRect(); \
         return {{ x: r.left + r.width / 2, y: r.top + r.height / 2 }}; }})()"
    );
    let at = evaluate(&js).await?;
    if at.is_null() {
        return Err(format!("no element with id {id}; call browser_snapshot again, ids change with the page"));
    }
    let x = at["x"].as_f64().ok_or("no x")?;
    let y = at["y"].as_f64().ok_or("no y")?;
    Ok((x, y))
}

async fn mouse(kind: &str, x: f64, y: f64) -> Result<(), String> {
    devtools::call(
        "Input.dispatchMouseEvent",
        json!({
            "type": kind,
            "x": x,
            "y": y,
            "button": "left",
            "clickCount": 1,
        }),
    )
    .await
    .map(|_| ())
}

/// A real click at the element's centre: move, press, release, through
/// Chromium's input path, so a page's own handlers see what a person's
/// click would give them.
async fn click(id: u64) -> Result<Value, String> {
    let (x, y) = locate(id).await?;
    mouse("mouseMoved", x, y).await?;
    mouse("mousePressed", x, y).await?;
    mouse("mouseReleased", x, y).await?;
    Ok(json!({ "clicked": id, "x": x, "y": y }))
}

/// Focus the element, select what it holds, and insert the text over it.
async fn type_text(id: u64, text: &str) -> Result<Value, String> {
    let js = format!(
        "(() => {{ const el = document.querySelector('[data-surya-id=\"{id}\"]'); \
         if (!el) return false; el.focus(); if (el.select) el.select(); return true; }})()"
    );
    if evaluate(&js).await? != Value::Bool(true) {
        return Err(format!("no element with id {id}; call browser_snapshot again"));
    }
    devtools::call("Input.insertText", json!({ "text": text })).await?;
    Ok(json!({ "typed": text.chars().count(), "into": id }))
}

/// The page as PNG, base64 as DevTools hands it over; decoded by whoever
/// writes it to disk or a card.
async fn screenshot() -> Result<Value, String> {
    let result = devtools::call("Page.captureScreenshot", json!({ "format": "png" })).await?;
    let data = result["data"].as_str().ok_or("Page.captureScreenshot returned no data")?;
    Ok(json!({ "png_base64": data, "mime": "image/png" }))
}

async fn eval(js: &str) -> Result<Value, String> {
    Ok(json!({ "value": evaluate(js).await? }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_come_as_numbers_or_numeric_strings() {
        assert_eq!(id_arg(&json!({ "id": 3 })), Ok(3));
        assert_eq!(id_arg(&json!({ "id": "12" })), Ok(12));
        assert!(id_arg(&json!({ "id": "x" })).is_err());
        assert!(id_arg(&json!({})).unwrap_err().contains("browser_snapshot"));
    }

    #[test]
    fn a_blank_string_argument_is_missing() {
        assert!(str_arg(&json!({ "url": "  " }), "url").is_err());
        assert_eq!(str_arg(&json!({ "url": "https://a" }), "url"), Ok("https://a"));
    }

    #[test]
    fn the_snapshot_script_is_an_expression_that_returns_lines() {
        // Runtime.evaluate wants one expression: the file is an IIFE.
        let js = SNAPSHOT_JS.trim();
        assert!(js.contains("(() => {") && js.ends_with("})()"));
        assert!(js.contains("data-surya-id"));
    }
}
