//! Per-card local state and the events a rendered card emits. The host
//! (comet's transcript) owns one [`CardState`] per card row, hands it to
//! the renderer read-only, and applies the [`CardEvent`]s the renderer's
//! click and key handlers send back.

use gpui::FocusHandle;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::images::{ImagePolicy, ResolvedImage};

/// Image decisions kept per card; the memo empties past this.
pub const MAX_IMAGE_MEMO: usize = 64;

use crate::data::{DataModel, Scope, resolve_value, value_to_bool, value_to_string};
use crate::model::{Action, Card, Dynamic, EventAction};
use crate::CardAction;

/// Where an input component keeps its value: a data-model path (two-way
/// binding, the spec's normal case) or, for a literal-valued input, a
/// per-component local slot so it still toggles and types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Binding {
    Path(String),
    Local(String),
}

/// What a card can do to its state, or ask the host to forward.
#[derive(Debug, Clone, PartialEq)]
pub enum CardEvent {
    /// A button press: `event.name` as `action`, the resolved context as
    /// `payload` — the host forwards it to the agent ([`CardAction::to_wire`]).
    /// `card_id` is the card the host gave the renderer, not the surface id.
    Action(CardAction),
    /// A local `functionCall` action (`openUrl` is the only useful one).
    Function { call: String, args: Value },
    /// An input wrote a value (checkbox toggle, text edit).
    SetValue { binding: Binding, value: Value },
    /// A tab header was pressed.
    SelectTab { component_id: String, index: usize },
    /// A text field asked for (or gave up) keyboard focus.
    Focus(Option<String>),
}

#[derive(Debug, Clone)]
pub struct CardState {
    pub data: DataModel,
    pub locals: HashMap<String, Value>,
    pub tabs: HashMap<String, usize>,
    /// The focused TextField's component id, if any.
    pub focused: Option<String>,
    /// One focus handle per card, lent to whichever field is focused. The
    /// host creates it (`cx.focus_handle()`); `None` means fields render
    /// but cannot take keyboard input.
    pub focus: Option<FocusHandle>,
    /// Bumped on every applied change — a cheap "re-measure me" signal.
    pub revision: u64,
    /// Image decisions by URL, filled on first render (see
    /// [`Self::image`]). Interior mutability because the renderer holds
    /// `&CardState`; cleared with the state on a card change.
    pub images: RefCell<HashMap<String, ResolvedImage>>,
}

impl CardState {
    pub fn new(card: &Card) -> Self {
        Self {
            data: DataModel::new(card.data.clone()),
            locals: HashMap::new(),
            tabs: HashMap::new(),
            focused: None,
            focus: None,
            revision: 0,
            images: RefCell::new(HashMap::new()),
        }
    }

    /// The memoized decision for `url` under `policy`; decided once. The
    /// memo holds at most [`MAX_IMAGE_MEMO`] URLs: an Image bound to a
    /// TextField's path would otherwise add one entry per keystroke.
    pub fn image(&self, url: &str, policy: &ImagePolicy) -> ResolvedImage {
        if let Some(hit) = self.images.borrow().get(url) {
            return hit.clone();
        }
        let resolved: ResolvedImage = policy.decide(url).into();
        let mut images = self.images.borrow_mut();
        if images.len() >= MAX_IMAGE_MEMO {
            images.clear();
        }
        images.insert(url.to_owned(), resolved.clone());
        resolved
    }

    pub fn with_focus(mut self, focus: FocusHandle) -> Self {
        self.focus = Some(focus);
        self
    }

    /// The binding an input component with `value` property `d` writes to.
    pub fn binding_for(component_id: &str, scope: &Scope, d: Option<&Dynamic<String>>) -> Binding {
        match d.and_then(Dynamic::path) {
            Some(p) => Binding::Path(scope.absolute(p)),
            None => Binding::Local(component_id.to_owned()),
        }
    }

    pub fn binding_for_bool(component_id: &str, scope: &Scope, d: &Dynamic<bool>) -> Binding {
        match d.path() {
            Some(p) => Binding::Path(scope.absolute(p)),
            None => Binding::Local(component_id.to_owned()),
        }
    }

    /// Read an input's current value: the local slot wins once written,
    /// else the bound path, else the literal from the component.
    pub fn read_string(&self, binding: &Binding, literal: Option<&Dynamic<String>>, scope: &Scope) -> String {
        match binding {
            Binding::Local(id) => match self.locals.get(id) {
                Some(v) => value_to_string(v),
                None => literal
                    .map(|d| crate::data::resolve_string(&self.data, scope, d))
                    .unwrap_or_default(),
            },
            Binding::Path(p) => self.data.get(p).map(value_to_string).unwrap_or_default(),
        }
    }

    pub fn read_bool(&self, binding: &Binding, literal: &Dynamic<bool>, scope: &Scope) -> bool {
        match binding {
            Binding::Local(id) => match self.locals.get(id) {
                Some(v) => value_to_bool(v),
                None => crate::data::resolve_bool(&self.data, scope, literal),
            },
            Binding::Path(p) => self.data.get(p).map(value_to_bool).unwrap_or(false),
        }
    }

    pub fn selected_tab(&self, component_id: &str) -> usize {
        self.tabs.get(component_id).copied().unwrap_or(0)
    }

    /// Apply a state-changing event. Returns whether anything changed (the
    /// host re-measures the row when it did). Actions and function calls
    /// are not state: they return `false` and the host handles them.
    pub fn apply(&mut self, event: &CardEvent) -> bool {
        let changed = match event {
            CardEvent::SetValue { binding, value } => match binding {
                Binding::Path(p) => self.data.set(p, value.clone()),
                Binding::Local(id) => {
                    self.locals.insert(id.clone(), value.clone());
                    true
                }
            },
            CardEvent::SelectTab {
                component_id,
                index,
            } => {
                let prev = self.tabs.insert(component_id.clone(), *index);
                prev != Some(*index)
            }
            CardEvent::Focus(id) => {
                let changed = self.focused != *id;
                self.focused = id.clone();
                changed
            }
            CardEvent::Action(_) | CardEvent::Function { .. } => false,
        };
        if changed {
            self.revision = self.revision.wrapping_add(1);
        }
        changed
    }

    /// Resolve a button's action against the current data model into the
    /// event the host forwards. `None` when the button has no action.
    pub fn resolve_action(&self, card: &Card, scope: &Scope, action: &Action) -> Option<CardEvent> {
        match action {
            Action::Event(EventAction { name, context }) => {
                let context = Value::Object(
                    context
                        .iter()
                        .map(|(k, v)| (k.clone(), resolve_value(&self.data, scope, v)))
                        .collect(),
                );
                Some(CardEvent::Action(CardAction {
                    card_id: card.card_id.clone(),
                    action: name.clone(),
                    payload: context,
                }))
            }
            Action::Function(call) => Some(CardEvent::Function {
                call: call.call.clone(),
                args: resolve_value(&self.data, scope, &Value::Object(call.args.clone())),
            }),
            Action::None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_card;
    use serde_json::json;

    fn card() -> Card {
        parse_card(&json!({
            "surfaceId": "follow_up",
            "components": [
                {"id": "root", "component": "Column", "children": ["who", "ok", "save"]},
                {"id": "who", "component": "TextField", "label": "Lead", "value": {"path": "/lead"}},
                {"id": "ok", "component": "CheckBox", "label": "Confirmed", "value": true},
                {"id": "save", "component": "Button", "child": "who", "action": {"event": {"name": "save", "context": {"lead": {"path": "/lead"}, "note": "x"}}}}
            ],
            "data": {"lead": "Ahmad"}
        }))
    }

    #[test]
    fn two_way_binding_writes_the_model_and_the_action_reads_it() {
        let card = card();
        let mut state = CardState::new(&card);
        let binding = Binding::Path("/lead".into());
        assert_eq!(state.read_string(&binding, None, &Scope::root()), "Ahmad");
        assert!(state.apply(&CardEvent::SetValue { binding: binding.clone(), value: json!("Mei") }));
        assert_eq!(state.read_string(&binding, None, &Scope::root()), "Mei");
        let action = match &card.get("save").unwrap().kind {
            crate::model::ComponentKind::Button { action, .. } => action.clone(),
            _ => unreachable!(),
        };
        let Some(CardEvent::Action(action)) = state.resolve_action(&card, &Scope::root(), &action) else {
            panic!("expected an action");
        };
        assert_eq!(action.card_id, "follow_up");
        assert_eq!(action.payload, json!({"lead": "Mei", "note": "x"}));
        assert_eq!(action.to_wire(), r#"[card:follow_up] save {"lead":"Mei","note":"x"}"#);
    }

    #[test]
    fn literal_inputs_toggle_locally() {
        let card = card();
        let mut state = CardState::new(&card);
        let literal = Dynamic::Literal(true);
        let binding = CardState::binding_for_bool("ok", &Scope::root(), &literal);
        assert_eq!(binding, Binding::Local("ok".into()));
        assert!(state.read_bool(&binding, &literal, &Scope::root()));
        state.apply(&CardEvent::SetValue { binding: binding.clone(), value: json!(false) });
        assert!(!state.read_bool(&binding, &literal, &Scope::root()));
        assert_eq!(state.revision, 1);
    }

    #[test]
    fn image_memo_is_bounded() {
        let card = card();
        let state = CardState::new(&card);
        let policy = ImagePolicy::default();
        for i in 0..(MAX_IMAGE_MEMO * 3) {
            state.image(&format!("https://h/{i}.png"), &policy);
            assert!(state.images.borrow().len() <= MAX_IMAGE_MEMO);
        }
        assert!(matches!(state.image("https://h/0.png", &policy), ResolvedImage::Placeholder(h) if h == "h"));
    }

    #[test]
    fn tabs_and_focus() {
        let card = card();
        let mut state = CardState::new(&card);
        assert!(state.apply(&CardEvent::SelectTab { component_id: "t".into(), index: 2 }));
        assert!(!state.apply(&CardEvent::SelectTab { component_id: "t".into(), index: 2 }));
        assert_eq!(state.selected_tab("t"), 2);
        assert!(state.apply(&CardEvent::Focus(Some("who".into()))));
        assert!(!state.apply(&CardEvent::Action(CardAction { card_id: "c".into(), action: "n".into(), payload: json!({}) })));
    }
}
