//! The data model: JSON Pointer reads and writes (RFC 6901, plus A2UI's
//! relative paths inside template scopes), `Dynamic` resolution with the
//! spec's type conversion, and the catalog functions a card actually uses.

use serde_json::Value;

use crate::model::{Dynamic, FunctionCall};

/// Highest array index a write may create. A pointer like `/x/99999999999`
/// would otherwise allocate that many nulls (the spec pads with null).
pub const MAX_ARRAY_INDEX: usize = 10_000;

/// A surface's data model. Absolute paths are JSON Pointers (`/a/b/0`);
/// `""` and `/` both mean the root.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DataModel {
    root: Value,
}

/// Where relative paths resolve: `None` is the root scope; inside a
/// template, `Some("/items/2")` is the current item.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    pub base: Option<String>,
}

impl Scope {
    pub fn root() -> Self {
        Scope { base: None }
    }

    pub fn item(base: impl Into<String>) -> Self {
        Scope {
            base: Some(base.into()),
        }
    }

    /// The absolute pointer for `path` in this scope.
    pub fn absolute(&self, path: &str) -> String {
        if path.is_empty() || path == "/" {
            return self.base.clone().unwrap_or_default();
        }
        if path.starts_with('/') {
            return path.to_owned();
        }
        match &self.base {
            Some(base) => format!("{base}/{path}"),
            None => format!("/{path}"),
        }
    }
}

impl DataModel {
    pub fn new(root: Value) -> Self {
        let root = if root.is_null() {
            Value::Object(Default::default())
        } else {
            root
        };
        Self { root }
    }

    pub fn as_value(&self) -> &Value {
        &self.root
    }

    pub fn into_value(self) -> Value {
        self.root
    }

    pub fn get(&self, pointer: &str) -> Option<&Value> {
        if pointer.is_empty() || pointer == "/" {
            return Some(&self.root);
        }
        self.root.pointer(pointer)
    }

    /// Upsert at `pointer`, creating intermediate objects (arrays when the
    /// next segment is a number). The root pointer replaces everything. A
    /// write that would pad an array past [`MAX_ARRAY_INDEX`] is refused
    /// (returns `false`, model untouched).
    pub fn set(&mut self, pointer: &str, value: Value) -> bool {
        if pointer.is_empty() || pointer == "/" {
            self.root = value;
            return true;
        }
        let segments: Vec<String> = pointer
            .trim_start_matches('/')
            .split('/')
            .map(unescape)
            .collect();
        if segments
            .iter()
            .any(|s| s.parse::<usize>().is_ok_and(|ix| ix > MAX_ARRAY_INDEX))
        {
            return false;
        }
        let mut cur = &mut self.root;
        for (ix, seg) in segments.iter().enumerate() {
            let last = ix + 1 == segments.len();
            let next_is_index = segments
                .get(ix + 1)
                .is_some_and(|s| s.parse::<usize>().is_ok());
            match seg.parse::<usize>() {
                Ok(index) if cur.is_array() || cur.is_null() => {
                    if !cur.is_array() {
                        *cur = Value::Array(Vec::new());
                    }
                    let arr = cur.as_array_mut().expect("array");
                    while arr.len() <= index {
                        arr.push(Value::Null);
                    }
                    if last {
                        arr[index] = value;
                        return true;
                    }
                    if !arr[index].is_object() && !arr[index].is_array() {
                        arr[index] = if next_is_index {
                            Value::Array(Vec::new())
                        } else {
                            Value::Object(Default::default())
                        };
                    }
                    cur = &mut arr[index];
                }
                _ => {
                    if !cur.is_object() {
                        *cur = Value::Object(Default::default());
                    }
                    let obj = cur.as_object_mut().expect("object");
                    if last {
                        obj.insert(seg.clone(), value);
                        return true;
                    }
                    let slot = obj.entry(seg.clone()).or_insert(Value::Null);
                    if !slot.is_object() && !slot.is_array() {
                        *slot = if next_is_index {
                            Value::Array(Vec::new())
                        } else {
                            Value::Object(Default::default())
                        };
                    }
                    cur = slot;
                }
            }
        }
        true
    }

    /// Remove the key at `pointer` (arrays: the slot becomes null, length
    /// kept, per spec). The root pointer empties the model.
    pub fn remove(&mut self, pointer: &str) {
        if pointer.is_empty() || pointer == "/" {
            self.root = Value::Object(Default::default());
            return;
        }
        let Some((parent, key)) = pointer.rsplit_once('/') else {
            return;
        };
        let key = unescape(key);
        let parent_ptr = if parent.is_empty() { "/" } else { parent };
        let Some(parent) = (if parent_ptr == "/" {
            Some(&mut self.root)
        } else {
            self.root.pointer_mut(parent_ptr)
        }) else {
            return;
        };
        match parent {
            Value::Object(obj) => {
                obj.remove(&key);
            }
            Value::Array(arr) => {
                if let Ok(ix) = key.parse::<usize>()
                    && ix < arr.len()
                {
                    arr[ix] = Value::Null;
                }
            }
            _ => {}
        }
    }
}

fn unescape(seg: &str) -> String {
    seg.replace("~1", "/").replace("~0", "~")
}

/// The spec's string conversion: numbers and booleans plain, null empty,
/// objects and arrays as JSON.
pub fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}

pub fn value_to_bool(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty() && s != "false",
        Value::Array(a) => !a.is_empty(),
        Value::Object(_) => true,
    }
}

pub fn resolve_string(model: &DataModel, scope: &Scope, d: &Dynamic<String>) -> String {
    match d {
        Dynamic::Literal(s) => s.clone(),
        Dynamic::Path(p) => model
            .get(&scope.absolute(p))
            .map(value_to_string)
            .unwrap_or_default(),
        Dynamic::Call(call) => value_to_string(&call_function(model, scope, call)),
    }
}

pub fn resolve_bool(model: &DataModel, scope: &Scope, d: &Dynamic<bool>) -> bool {
    match d {
        Dynamic::Literal(b) => *b,
        Dynamic::Path(p) => model
            .get(&scope.absolute(p))
            .map(value_to_bool)
            .unwrap_or(false),
        Dynamic::Call(call) => value_to_bool(&call_function(model, scope, call)),
    }
}

/// Resolve a `DynamicValue` as it appears in action contexts and function
/// args: `{path}` reads the model, `{call, args}` runs a function, objects
/// and arrays resolve their members, everything else is literal.
pub fn resolve_value(model: &DataModel, scope: &Scope, v: &Value) -> Value {
    match v {
        Value::Object(o) => {
            if let Some(path) = o.get("path").and_then(Value::as_str)
                && o.len() == 1
            {
                return model.get(&scope.absolute(path)).cloned().unwrap_or(Value::Null);
            }
            if let Some(call) = crate::parse::parse_call(o) {
                return call_function(model, scope, &call);
            }
            Value::Object(
                o.iter()
                    .map(|(k, v)| (k.clone(), resolve_value(model, scope, v)))
                    .collect(),
            )
        }
        Value::Array(a) => Value::Array(a.iter().map(|v| resolve_value(model, scope, v)).collect()),
        other => other.clone(),
    }
}

/// The catalog functions a card renders with. Validation functions return
/// booleans; unknown calls resolve to null (an empty string on screen).
pub fn call_function(model: &DataModel, scope: &Scope, call: &FunctionCall) -> Value {
    let arg = |key: &str| {
        call.args
            .get(key)
            .map(|v| resolve_value(model, scope, v))
            .unwrap_or(Value::Null)
    };
    match call.call.as_str() {
        "formatString" => Value::String(format_string(model, scope, &value_to_string(&arg("value")))),
        "formatNumber" => {
            let n = arg("value").as_f64().unwrap_or(0.0);
            let decimals = arg("decimals").as_u64().map(|d| d as usize);
            let grouping = arg("grouping").as_bool().unwrap_or(true);
            Value::String(format_number(n, decimals, grouping))
        }
        "formatCurrency" => {
            let n = arg("value").as_f64().unwrap_or(0.0);
            let code = value_to_string(&arg("currency"));
            let body = format_number(n, Some(2), true);
            Value::String(if code.is_empty() { body } else { format!("{code} {body}") })
        }
        "required" => Value::Bool(match arg("value") {
            Value::Null => false,
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            _ => true,
        }),
        "not" => Value::Bool(!value_to_bool(&arg("value"))),
        "and" | "or" => {
            let values = arg("values");
            let items = values.as_array().cloned().unwrap_or_default();
            let all = items.iter().map(value_to_bool);
            Value::Bool(if call.call == "and" {
                all.clone().all(|b| b)
            } else {
                all.clone().any(|b| b)
            })
        }
        "length" => {
            let s = value_to_string(&arg("value"));
            let len = s.chars().count() as u64;
            let min = arg("min").as_u64().unwrap_or(0);
            let max = arg("max").as_u64().unwrap_or(u64::MAX);
            Value::Bool(len >= min && len <= max)
        }
        "numeric" => {
            let Some(n) = arg("value").as_f64().or_else(|| {
                value_to_string(&arg("value")).parse::<f64>().ok()
            }) else {
                return Value::Bool(false);
            };
            let min = arg("min").as_f64().unwrap_or(f64::NEG_INFINITY);
            let max = arg("max").as_f64().unwrap_or(f64::INFINITY);
            Value::Bool(n >= min && n <= max)
        }
        "email" => {
            let s = value_to_string(&arg("value"));
            let ok = s.split_once('@').is_some_and(|(local, domain)| {
                !local.is_empty()
                    && domain.contains('.')
                    && !domain.starts_with('.')
                    && !domain.ends_with('.')
                    && !s.chars().any(char::is_whitespace)
            });
            Value::Bool(ok)
        }
        _ => Value::Null,
    }
}

/// `${/abs/path}` and `${relative}` interpolation with `\${` escapes.
/// Function-call expressions inside `${...}` are not evaluated in v1; they
/// render empty rather than as raw source.
pub fn format_string(model: &DataModel, scope: &Scope, template: &str) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("${") {
        let escaped = rest[..start].ends_with('\\');
        out.push_str(&rest[..start]);
        if escaped {
            out.pop();
            out.push_str("${");
            rest = &rest[start + 2..];
            continue;
        }
        let Some(end) = rest[start..].find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let expr = rest[start + 2..start + end].trim();
        if !expr.contains('(') {
            out.push_str(
                &model
                    .get(&scope.absolute(expr))
                    .map(value_to_string)
                    .unwrap_or_default(),
            );
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    out
}

fn format_number(n: f64, decimals: Option<usize>, grouping: bool) -> String {
    let decimals = decimals.unwrap_or(if n.fract() == 0.0 { 0 } else { 2 });
    let s = format!("{n:.decimals$}");
    let (int, frac) = s.split_once('.').map_or((s.as_str(), ""), |(i, f)| (i, f));
    let (sign, digits) = int
        .strip_prefix('-')
        .map_or(("", int), |d| ("-", d));
    let grouped = if grouping {
        let mut out = String::new();
        for (ix, ch) in digits.chars().enumerate() {
            if ix > 0 && (digits.len() - ix) % 3 == 0 {
                out.push(',');
            }
            out.push(ch);
        }
        out
    } else {
        digits.to_owned()
    };
    if frac.is_empty() {
        format!("{sign}{grouped}")
    } else {
        format!("{sign}{grouped}.{frac}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pointer_get_set_remove() {
        let mut m = DataModel::new(json!({"user": {"name": "Ann"}}));
        assert_eq!(m.get("/user/name"), Some(&json!("Ann")));
        m.set("/user/age", json!(31));
        m.set("/tags/1", json!("b"));
        assert_eq!(m.get("/tags"), Some(&json!([null, "b"])));
        m.remove("/user/name");
        assert_eq!(m.get("/user"), Some(&json!({"age": 31})));
        m.set("/", json!({"fresh": true}));
        assert_eq!(m.as_value(), &json!({"fresh": true}));
    }

    #[test]
    fn huge_array_index_is_refused_not_allocated() {
        let mut m = DataModel::new(json!({"x": [1]}));
        assert!(!m.set("/x/99999999999", json!(1)));
        assert!(!m.set("/y/10001/z", json!(1)));
        assert_eq!(m.as_value(), &json!({"x": [1]}));
        assert!(m.set("/x/3", json!(4)));
        assert_eq!(m.get("/x"), Some(&json!([1, null, null, 4])));
    }

    #[test]
    fn relative_scope_and_format_string() {
        let m = DataModel::new(json!({"company": "Acme", "people": [{"name": "Bo"}]}));
        let scope = &Scope::item("/people/0");
        assert_eq!(scope.absolute("name"), "/people/0/name");
        assert_eq!(
            format_string(&m, scope, "${name} at ${/company} \\${literal}"),
            "Bo at Acme ${literal}"
        );
        assert_eq!(
            resolve_string(&m, scope, &Dynamic::Path("name".into())),
            "Bo"
        );
    }

    #[test]
    fn functions() {
        let m = DataModel::new(json!({"n": 1234567.891, "e": "a@b.co"}));
        let call = |name: &str, args: Value| FunctionCall {
            call: name.into(),
            args: args.as_object().cloned().unwrap_or_default(),
        };
        assert_eq!(
            call_function(&m, &Scope::root(), &call("formatNumber", json!({"value": {"path": "/n"}, "decimals": 1}))),
            json!("1,234,567.9")
        );
        assert_eq!(
            call_function(&m, &Scope::root(), &call("formatCurrency", json!({"value": 12.5, "currency": "RM"}))),
            json!("RM 12.50")
        );
        assert_eq!(call_function(&m, &Scope::root(), &call("email", json!({"value": {"path": "/e"}}))), json!(true));
        assert_eq!(call_function(&m, &Scope::root(), &call("required", json!({"value": ""}))), json!(false));
        assert_eq!(call_function(&m, &Scope::root(), &call("mystery", json!({}))), Value::Null);
    }
}
