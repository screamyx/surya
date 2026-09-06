//! The six built-in card shapes (decision 14) expanded into A2UI v0.9.1
//! envelope messages over the basic catalog.
//!
//! An agent writes the short form — `{"shape":"metric","title":…,"value":…}` —
//! and this module turns it into the `createSurface` / `updateComponents` /
//! `updateDataModel` stream the renderer consumes. An agent that wants full
//! control skips the short form and passes A2UI messages straight through
//! (see [`crate::cards::normalize`]).
//!
//! Component and property names are taken from the A2UI basic catalog at
//! `specification/v0_9_1/catalogs/basic/catalog.json`.

use serde_json::{Map, Value, json};

pub const PROTOCOL_VERSION: &str = "v0.9.1";
pub const DEFAULT_CATALOG_ID: &str = "https://a2ui.org/specification/v0_9/catalogs/basic/catalog.json";

/// The shapes surya renders without a workspace catalog.
pub const BUILT_IN_SHAPES: &[(&str, &str)] = &[
    (
        "record",
        "One thing with its fields: title, optional image, label/value rows, badges, actions.",
    ),
    (
        "table",
        "Rows and columns. Use when the answer is a list the user will scan.",
    ),
    (
        "form",
        "Fields the user fills in and submits. Text, select and date fields.",
    ),
    (
        "approval",
        "A thing about to happen, its summary and the code or command, with approve and reject.",
    ),
    (
        "diff-summary",
        "Which files changed and by how much, with a one-line note.",
    ),
    (
        "metric",
        "One number with its change and an optional short series.",
    ),
];

/// Collects components while ids stay unique inside one surface.
struct Tree {
    components: Vec<Value>,
    seq: usize,
}

impl Tree {
    fn new() -> Self {
        Self {
            components: Vec::new(),
            seq: 0,
        }
    }

    fn next_id(&mut self, prefix: &str) -> String {
        self.seq += 1;
        format!("{prefix}_{}", self.seq)
    }

    /// Push a component under a generated id and return that id.
    fn add(&mut self, prefix: &str, mut body: Map<String, Value>) -> String {
        let id = self.next_id(prefix);
        body.insert("id".into(), Value::String(id.clone()));
        self.components.push(Value::Object(body));
        id
    }

    fn text(&mut self, text: &str, variant: &str) -> String {
        let mut body = Map::new();
        body.insert("component".into(), json!("Text"));
        body.insert("text".into(), json!(text));
        body.insert("variant".into(), json!(variant));
        self.add("text", body)
    }

    /// A table cell. The weight is what makes the row a set of columns: the
    /// renderer wraps every weightless child of a horizontal container in
    /// `flex_none`, so without it each cell is only as wide as its own text
    /// and nothing lines up between rows. The hand-written reference table
    /// carries the same `weight: 1` on every cell
    /// (`a2ui/fixtures/02-table.json`).
    fn cell(&mut self, text: &str, variant: &str) -> String {
        let mut body = Map::new();
        body.insert("component".into(), json!("Text"));
        body.insert("text".into(), json!(text));
        body.insert("variant".into(), json!(variant));
        body.insert("weight".into(), json!(1));
        self.add("text", body)
    }

    fn column(&mut self, children: Vec<String>) -> String {
        let mut body = Map::new();
        body.insert("component".into(), json!("Column"));
        body.insert("children".into(), json!(children));
        body.insert("align".into(), json!("stretch"));
        self.add("col", body)
    }

    fn row(&mut self, children: Vec<String>, justify: &str) -> String {
        let mut body = Map::new();
        body.insert("component".into(), json!("Row"));
        body.insert("children".into(), json!(children));
        body.insert("justify".into(), json!(justify));
        body.insert("align".into(), json!("center"));
        self.add("row", body)
    }

    fn divider(&mut self) -> String {
        let mut body = Map::new();
        body.insert("component".into(), json!("Divider"));
        body.insert("axis".into(), json!("horizontal"));
        self.add("rule", body)
    }

    fn button(&mut self, label: &str, variant: &str, event: &str, context: Value) -> String {
        let child = self.text(label, "body");
        let mut body = Map::new();
        body.insert("component".into(), json!("Button"));
        body.insert("child".into(), json!(child));
        body.insert("variant".into(), json!(variant));
        body.insert(
            "action".into(),
            json!({ "event": { "name": event, "context": context } }),
        );
        self.add("button", body)
    }

    /// Wrap the built children in a Card and name it `root`, as the envelope
    /// requires ("One of the components ... MUST have an `id` of `root`").
    fn finish(mut self, body_id: String) -> Vec<Value> {
        self.components.push(json!({
            "id": "root",
            "component": "Card",
            "child": body_id,
        }));
        self.components
    }
}

fn str_at<'a>(card: &'a Value, key: &str) -> &'a str {
    card.get(key).and_then(Value::as_str).unwrap_or_default()
}

fn array_at<'a>(card: &'a Value, key: &str) -> &'a [Value] {
    card.get(key).and_then(Value::as_array).map_or(&[], |v| v)
}

/// Render `value` as the plain text a Text component shows.
fn scalar(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Expand one short-form card into its component list plus the data model the
/// surface starts with. Returns `Err` with a message the agent can act on.
pub fn expand(card: &Value) -> Result<(Vec<Value>, Value), String> {
    let shape = str_at(card, "shape");
    let mut tree = Tree::new();
    let title = str_at(card, "title");
    let mut body: Vec<String> = Vec::new();
    if !title.is_empty() {
        body.push(tree.text(title, "h3"));
    }
    let mut data = json!({});

    match shape {
        "record" => record(&mut tree, card, &mut body),
        "table" => table(&mut tree, card, &mut body),
        "form" => data = form(&mut tree, card, &mut body),
        "approval" => approval(&mut tree, card, &mut body),
        "diff-summary" => diff_summary(&mut tree, card, &mut body),
        "metric" => data = metric(&mut tree, card, &mut body),
        "" => return Err("card is missing \"shape\"; use one of: record, table, form, approval, diff-summary, metric - or pass A2UI messages instead".into()),
        other => {
            return Err(format!(
                "unknown shape \"{other}\"; built-in shapes are record, table, form, approval, diff-summary, metric. Call list_cards to see this workspace's own cards."
            ));
        }
    }

    let body_id = tree.column(body);
    Ok((tree.finish(body_id), data))
}

fn record(tree: &mut Tree, card: &Value, body: &mut Vec<String>) {
    let subtitle = str_at(card, "subtitle");
    if !subtitle.is_empty() {
        body.push(tree.text(subtitle, "caption"));
    }
    let image = str_at(card, "image");
    if !image.is_empty() {
        let mut img = Map::new();
        img.insert("component".into(), json!("Image"));
        img.insert("url".into(), json!(image));
        img.insert("description".into(), json!(str_at(card, "title")));
        img.insert("fit".into(), json!("cover"));
        img.insert("variant".into(), json!("mediumFeature"));
        body.push(tree.add("image", img));
    }
    for badge in array_at(card, "badges") {
        let text = scalar(badge);
        if !text.is_empty() {
            body.push(tree.text(&text, "caption"));
        }
    }
    for field in array_at(card, "fields") {
        let label = tree.text(str_at(field, "label"), "caption");
        let value = tree.text(&scalar(field.get("value").unwrap_or(&Value::Null)), "body");
        body.push(tree.row(vec![label, value], "spaceBetween"));
    }
    actions(tree, card, body);
}

fn table(tree: &mut Tree, card: &Value, body: &mut Vec<String>) {
    let columns = array_at(card, "columns");
    if !columns.is_empty() {
        let cells: Vec<String> = columns
            .iter()
            .map(|c| tree.cell(&scalar(c), "caption"))
            .collect();
        body.push(tree.row(cells, "start"));
        body.push(tree.divider());
    }
    for row in array_at(card, "rows") {
        let Some(cells) = row.as_array() else { continue };
        let cells: Vec<String> = cells
            .iter()
            .map(|cell| {
                let text = scalar(cell);
                tree.cell(&text, "body")
            })
            .collect();
        if !cells.is_empty() {
            body.push(tree.row(cells, "start"));
        }
    }
    actions(tree, card, body);
}

/// Fields bind two-way into `/form/<key>`; the seeded values ride the
/// surface's data model so the renderer shows them before any edit.
fn form(tree: &mut Tree, card: &Value, body: &mut Vec<String>) -> Value {
    let mut seed = Map::new();
    for (index, field) in array_at(card, "fields").iter().enumerate() {
        let label = str_at(field, "label");
        let key = match field.get("key").and_then(Value::as_str) {
            Some(key) if !key.is_empty() => key.to_string(),
            _ => format!("field{}", index + 1),
        };
        let path = format!("/form/{key}");
        let value = field.get("value").cloned().unwrap_or(Value::Null);
        seed.insert(key, value.clone());
        let kind = str_at(field, "type");
        let mut input = Map::new();
        match kind {
            "select" => {
                input.insert("component".into(), json!("ChoicePicker"));
                input.insert("label".into(), json!(label));
                input.insert("variant".into(), json!("mutuallyExclusive"));
                let options: Vec<Value> = array_at(field, "options")
                    .iter()
                    .map(|o| json!({ "label": scalar(o), "value": scalar(o) }))
                    .collect();
                input.insert("options".into(), Value::Array(options));
                input.insert("value".into(), json!({ "path": path }));
            }
            "date" => {
                input.insert("component".into(), json!("DateTimeInput"));
                input.insert("value".into(), json!({ "path": path }));
                input.insert("enableDate".into(), json!(true));
                input.insert("enableTime".into(), json!(false));
                body.push(tree.text(label, "caption"));
            }
            _ => {
                input.insert("component".into(), json!("TextField"));
                input.insert("label".into(), json!(label));
                input.insert("value".into(), json!({ "path": path }));
                input.insert("variant".into(), json!("shortText"));
            }
        }
        body.push(tree.add("input", input));
    }
    let submit = match card.get("submit").and_then(Value::as_str) {
        Some(label) if !label.is_empty() => label,
        _ => "Submit",
    };
    let event = match card.get("event").and_then(Value::as_str) {
        Some(event) if !event.is_empty() => event,
        _ => "surya_form_submit",
    };
    body.push(tree.button(submit, "primary", event, json!({ "form": { "path": "/form" } })));
    json!({ "form": Value::Object(seed) })
}

fn approval(tree: &mut Tree, card: &Value, body: &mut Vec<String>) {
    let summary = str_at(card, "summary");
    if !summary.is_empty() {
        body.push(tree.text(summary, "body"));
    }
    let code = str_at(card, "code");
    if !code.is_empty() {
        // Text supports simple Markdown, so a fenced block keeps the code
        // monospaced without a catalog component for it.
        body.push(tree.text(&format!("```\n{code}\n```"), "body"));
    }
    let approve = card
        .get("approve")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("Approve");
    let reject = card
        .get("reject")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("Reject");
    let context = card.get("context").cloned().unwrap_or_else(|| json!({}));
    let yes = tree.button(approve, "primary", "surya_approve", context.clone());
    let no = tree.button(reject, "borderless", "surya_reject", context);
    body.push(tree.row(vec![yes, no], "start"));
}

fn diff_summary(tree: &mut Tree, card: &Value, body: &mut Vec<String>) {
    for file in array_at(card, "files") {
        let path = tree.text(str_at(file, "path"), "body");
        let added = file.get("added").and_then(Value::as_i64).unwrap_or(0);
        let removed = file.get("removed").and_then(Value::as_i64).unwrap_or(0);
        let counts = tree.text(&format!("+{added} -{removed}"), "caption");
        body.push(tree.row(vec![path, counts], "spaceBetween"));
    }
    let note = str_at(card, "note");
    if !note.is_empty() {
        body.push(tree.divider());
        body.push(tree.text(note, "caption"));
    }
    actions(tree, card, body);
}

/// The series has no component in the basic catalog, so it rides the data
/// model at `/series` for a renderer that can draw it, with a caption as the
/// fallback a plain basic-catalog client still shows.
fn metric(tree: &mut Tree, card: &Value, body: &mut Vec<String>) -> Value {
    let value = str_at(card, "value");
    if !value.is_empty() {
        body.push(tree.text(value, "h1"));
    }
    let delta = str_at(card, "delta");
    if !delta.is_empty() {
        body.push(tree.text(delta, "caption"));
    }
    let series: Vec<Value> = array_at(card, "series").to_vec();
    if !series.is_empty() {
        let printed: Vec<String> = series.iter().map(scalar).collect();
        body.push(tree.text(&printed.join("  "), "caption"));
    }
    actions(tree, card, body);
    if series.is_empty() {
        json!({})
    } else {
        json!({ "series": Value::Array(series) })
    }
}

/// `actions: [{ "label": "Open", "event": "open", "context": { … } }]` —
/// every shape accepts them, so a card is never a dead end.
fn actions(tree: &mut Tree, card: &Value, body: &mut Vec<String>) {
    let listed = array_at(card, "actions");
    if listed.is_empty() {
        return;
    }
    let mut buttons = Vec::new();
    for (index, action) in listed.iter().enumerate() {
        let label = str_at(action, "label");
        if label.is_empty() {
            continue;
        }
        let event = match action.get("event").and_then(Value::as_str) {
            Some(event) if !event.is_empty() => event,
            _ => "surya_card_action",
        };
        let context = action.get("context").cloned().unwrap_or_else(|| json!({}));
        let variant = if index == 0 { "primary" } else { "borderless" };
        buttons.push(tree.button(label, variant, event, context));
    }
    if !buttons.is_empty() {
        body.push(tree.row(buttons, "start"));
    }
}

#[cfg(test)]
mod tests;
