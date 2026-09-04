//! A2UI message shape → [`Card`]. Tolerant by design: every failure becomes
//! a diagnostic on the card, never a panic, because the input is model
//! output and a mistyped property must not take the transcript down.

use serde_json::Value;

use crate::data::DataModel;
use crate::model::*;

/// Parse a card from JSON text: one envelope, an array of envelopes, the
/// shorthand object, or JSONL (one envelope per line).
pub fn parse_card_str(text: &str) -> Card {
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return parse_card(&value);
    }
    // JSONL: one envelope per non-empty line.
    let mut lines = Vec::new();
    let mut errors = Vec::new();
    for (ix, line) in trimmed.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(v) => lines.push(v),
            Err(e) => errors.push(format!("line {}: {e}", ix + 1)),
        }
    }
    let mut card = parse_card(&Value::Array(lines));
    card.errors.splice(0..0, errors);
    card
}

/// Parse a card from a JSON value. Accepted shapes:
/// - an array (or `{"messages": [...]}`) of A2UI envelopes —
///   `createSurface`, `updateComponents`, `updateDataModel`;
/// - a single envelope object;
/// - the shorthand `{surfaceId?, catalogId?, components: [...], data?: {}}`
///   a `show_card` tool call carries;
/// - a JSON string holding any of the above.
pub fn parse_card(input: &Value) -> Card {
    let mut card = Card {
        id: "card".to_owned(),
        catalog_id: String::new(),
        components: Default::default(),
        data: Value::Object(Default::default()),
        errors: Vec::new(),
    };
    match input {
        Value::String(s) => return parse_card_str(s),
        Value::Array(items) => {
            for item in items {
                apply_envelope(&mut card, item);
            }
        }
        Value::Object(obj) => {
            if let Some(Value::Array(items)) = obj.get("messages") {
                for item in items {
                    apply_envelope(&mut card, item);
                }
            } else if is_envelope(obj) {
                apply_envelope(&mut card, input);
            } else {
                apply_shorthand(&mut card, obj);
            }
        }
        other => card.errors.push(format!("card must be an object or array, got {other}")),
    }
    if card.components.is_empty() {
        card.errors.push("no components".to_owned());
    } else if card.root().is_none() {
        card.errors
            .push(format!("no component with id \"{ROOT_ID}\""));
    }
    card
}

fn is_envelope(obj: &serde_json::Map<String, Value>) -> bool {
    ["createSurface", "updateComponents", "updateDataModel", "deleteSurface"]
        .iter()
        .any(|k| obj.contains_key(*k))
}

fn apply_envelope(card: &mut Card, item: &Value) {
    let Some(obj) = item.as_object() else {
        card.errors.push(format!("envelope must be an object, got {item}"));
        return;
    };
    if let Some(create) = obj.get("createSurface") {
        if let Some(id) = str_field(create, "surfaceId") {
            card.id = id;
        }
        if let Some(catalog) = str_field(create, "catalogId") {
            card.catalog_id = catalog;
        }
    } else if let Some(update) = obj.get("updateComponents") {
        if let Some(id) = str_field(update, "surfaceId")
            && card.catalog_id.is_empty()
            && card.id == "card"
        {
            card.id = id;
        }
        match update.get("components").and_then(Value::as_array) {
            Some(items) => add_components(card, items),
            None => card.errors.push("updateComponents without a components array".into()),
        }
    } else if let Some(update) = obj.get("updateDataModel") {
        let path = str_field(update, "path").unwrap_or_default();
        let mut model = DataModel::new(std::mem::take(&mut card.data));
        match update.get("value") {
            Some(value) => model.set(&path, value.clone()),
            None => model.remove(&path),
        }
        card.data = model.into_value();
    } else if obj.contains_key("deleteSurface") {
        // A delete inside a single card payload is meaningless; ignore.
    } else if obj.contains_key("components") {
        apply_shorthand(card, obj);
    } else {
        card.errors.push(format!(
            "unknown envelope keys: {}",
            obj.keys().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
}

fn apply_shorthand(card: &mut Card, obj: &serde_json::Map<String, Value>) {
    if let Some(id) = obj
        .get("surfaceId")
        .or_else(|| obj.get("id"))
        .and_then(Value::as_str)
    {
        card.id = id.to_owned();
    }
    if let Some(catalog) = obj.get("catalogId").and_then(Value::as_str) {
        card.catalog_id = catalog.to_owned();
    }
    match obj.get("components").and_then(Value::as_array) {
        Some(items) => add_components(card, items),
        None => card.errors.push("shorthand card without a components array".into()),
    }
    if let Some(data) = obj.get("data").or_else(|| obj.get("dataModel")) {
        card.data = data.clone();
    }
}

fn add_components(card: &mut Card, items: &[Value]) {
    for (ix, item) in items.iter().enumerate() {
        match parse_component(item) {
            Ok(component) => {
                card.components.insert(component.id.clone(), component);
            }
            Err(e) => card.errors.push(format!("components[{ix}]: {e}")),
        }
    }
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_owned)
}

/// One component object → [`Component`]. Only `id` and `component` are
/// required to exist; a missing required property degrades to a default so
/// the tree still renders (the spec asks for progressive rendering of
/// partial trees).
pub fn parse_component(v: &Value) -> Result<Component, String> {
    let obj = v.as_object().ok_or_else(|| format!("expected an object, got {v}"))?;
    let id = obj
        .get("id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or("missing id")?
        .to_owned();
    let name = obj
        .get("component")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{id}: missing component name"))?;
    let kind = match name {
        "Text" => ComponentKind::Text {
            text: dyn_string(obj, "text").unwrap_or(Dynamic::Literal(String::new())),
            variant: match enum_str(obj, "variant") {
                "h1" => TextVariant::H1,
                "h2" => TextVariant::H2,
                "h3" => TextVariant::H3,
                "h4" => TextVariant::H4,
                "h5" => TextVariant::H5,
                "caption" => TextVariant::Caption,
                _ => TextVariant::Body,
            },
        },
        "Image" => ComponentKind::Image {
            url: dyn_string(obj, "url").unwrap_or(Dynamic::Literal(String::new())),
            fit: match enum_str(obj, "fit") {
                "contain" => ImageFit::Contain,
                "cover" => ImageFit::Cover,
                "none" => ImageFit::None,
                "scaleDown" => ImageFit::ScaleDown,
                _ => ImageFit::Fill,
            },
            variant: match enum_str(obj, "variant") {
                "icon" => ImageVariant::Icon,
                "avatar" => ImageVariant::Avatar,
                "smallFeature" => ImageVariant::SmallFeature,
                "largeFeature" => ImageVariant::LargeFeature,
                "header" => ImageVariant::Header,
                _ => ImageVariant::MediumFeature,
            },
            description: dyn_string(obj, "description"),
        },
        "Button" => ComponentKind::Button {
            child: child_id(obj, "child").unwrap_or_default(),
            variant: match enum_str(obj, "variant") {
                "primary" => ButtonVariant::Primary,
                "borderless" => ButtonVariant::Borderless,
                _ => ButtonVariant::Default,
            },
            action: parse_action(obj.get("action")),
        },
        "TextField" => ComponentKind::TextField {
            label: dyn_string(obj, "label").unwrap_or(Dynamic::Literal(String::new())),
            value: dyn_string(obj, "value"),
            variant: match enum_str(obj, "variant") {
                "longText" => TextFieldVariant::LongText,
                "number" => TextFieldVariant::Number,
                "obscured" => TextFieldVariant::Obscured,
                _ => TextFieldVariant::ShortText,
            },
        },
        "CheckBox" => ComponentKind::CheckBox {
            label: dyn_string(obj, "label").unwrap_or(Dynamic::Literal(String::new())),
            value: dyn_bool(obj, "value").unwrap_or(Dynamic::Literal(false)),
        },
        "Row" => ComponentKind::Row {
            children: child_list(obj, "children"),
            justify: parse_justify(enum_str(obj, "justify")),
            align: parse_align(enum_str(obj, "align")),
        },
        "Column" => ComponentKind::Column {
            children: child_list(obj, "children"),
            justify: parse_justify(enum_str(obj, "justify")),
            align: parse_align(enum_str(obj, "align")),
        },
        "List" => ComponentKind::List {
            children: child_list(obj, "children"),
            direction: match enum_str(obj, "direction") {
                "horizontal" => Axis::Horizontal,
                _ => Axis::Vertical,
            },
            align: parse_align(enum_str(obj, "align")),
        },
        "Card" => ComponentKind::Card {
            child: child_id(obj, "child").unwrap_or_default(),
        },
        "Divider" => ComponentKind::Divider {
            axis: match enum_str(obj, "axis") {
                "vertical" => Axis::Vertical,
                _ => Axis::Horizontal,
            },
        },
        "Tabs" => ComponentKind::Tabs {
            tabs: obj
                .get("tabs")
                .and_then(Value::as_array)
                .map(|tabs| {
                    tabs.iter()
                        .filter_map(|t| {
                            let tab = t.as_object()?;
                            Some(Tab {
                                title: dyn_string(tab, "title")
                                    .unwrap_or(Dynamic::Literal("Tab".into())),
                                child: child_id(tab, "child")?,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
        },
        other => ComponentKind::Unknown {
            name: other.to_owned(),
            raw: v.clone(),
        },
    };
    let weight = obj.get("weight").and_then(Value::as_f64).map(|w| w as f32);
    Ok(Component { id, kind, weight })
}

fn enum_str<'a>(obj: &'a serde_json::Map<String, Value>, key: &str) -> &'a str {
    obj.get(key).and_then(Value::as_str).unwrap_or("")
}

fn parse_justify(s: &str) -> Justify {
    match s {
        "center" => Justify::Center,
        "end" => Justify::End,
        "spaceBetween" => Justify::SpaceBetween,
        "spaceAround" => Justify::SpaceAround,
        "spaceEvenly" => Justify::SpaceEvenly,
        "stretch" => Justify::Stretch,
        _ => Justify::Start,
    }
}

fn parse_align(s: &str) -> Align {
    match s {
        "start" => Align::Start,
        "center" => Align::Center,
        "end" => Align::End,
        _ => Align::Stretch,
    }
}

fn child_id(obj: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    obj.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn child_list(obj: &serde_json::Map<String, Value>, key: &str) -> ChildList {
    match obj.get(key) {
        Some(Value::Array(ids)) => ChildList::Static(
            ids.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
        ),
        Some(Value::Object(template)) => match (
            template.get("componentId").and_then(Value::as_str),
            template.get("path").and_then(Value::as_str),
        ) {
            (Some(component_id), Some(path)) => ChildList::Template {
                component_id: component_id.to_owned(),
                path: path.to_owned(),
            },
            _ => ChildList::Static(Vec::new()),
        },
        _ => ChildList::Static(Vec::new()),
    }
}

/// A `DynamicString`: literal string (numbers and booleans are accepted
/// and stringified — models do that), `{path}`, or `{call, args}`.
fn dyn_string(obj: &serde_json::Map<String, Value>, key: &str) -> Option<Dynamic<String>> {
    match obj.get(key)? {
        Value::String(s) => Some(Dynamic::Literal(s.clone())),
        Value::Number(n) => Some(Dynamic::Literal(n.to_string())),
        Value::Bool(b) => Some(Dynamic::Literal(b.to_string())),
        Value::Object(o) => dyn_object(o),
        _ => None,
    }
}

fn dyn_bool(obj: &serde_json::Map<String, Value>, key: &str) -> Option<Dynamic<bool>> {
    match obj.get(key)? {
        Value::Bool(b) => Some(Dynamic::Literal(*b)),
        Value::String(s) => Some(Dynamic::Literal(s == "true")),
        Value::Object(o) => dyn_object(o),
        _ => None,
    }
}

fn dyn_object<T>(o: &serde_json::Map<String, Value>) -> Option<Dynamic<T>> {
    if let Some(path) = o.get("path").and_then(Value::as_str) {
        return Some(Dynamic::Path(path.to_owned()));
    }
    parse_call(o).map(Dynamic::Call)
}

pub(crate) fn parse_call(o: &serde_json::Map<String, Value>) -> Option<FunctionCall> {
    let call = o.get("call").and_then(Value::as_str)?;
    Some(FunctionCall {
        call: call.to_owned(),
        args: o
            .get("args")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default(),
    })
}

fn parse_action(v: Option<&Value>) -> Action {
    let Some(obj) = v.and_then(Value::as_object) else {
        return Action::None;
    };
    if let Some(event) = obj.get("event").and_then(Value::as_object) {
        let Some(name) = event.get("name").and_then(Value::as_str) else {
            return Action::None;
        };
        return Action::Event(EventAction {
            name: name.to_owned(),
            context: event
                .get("context")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default(),
        });
    }
    if let Some(call) = obj.get("functionCall").and_then(Value::as_object) {
        if let Some(call) = parse_call(call) {
            return Action::Function(call);
        }
    }
    // A bare `{name, context}` is a common model shortcut.
    if let Some(name) = obj.get("name").and_then(Value::as_str) {
        return Action::Event(EventAction {
            name: name.to_owned(),
            context: obj
                .get("context")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default(),
        });
    }
    Action::None
}
