//! A readable projection of card replies; the stored agent protocol stays intact.
use super::{Row, RowKind, fnv1a};
use std::collections::HashMap;
use std::sync::Arc;
use surya_a2ui::data::{Scope, resolve_string};
use surya_a2ui::model::ChildList;
use surya_a2ui::{Card, CardEvent, CardState, ComponentKind};
use surya_proto::CardAction;

/// Project after per-entry caching: a reply's label belongs to an earlier
/// card, so changing that card must also invalidate the acknowledgement.
pub(super) fn project(rows: &mut [Row]) {
    let mut cards: HashMap<String, Arc<Card>> = HashMap::new();
    for row in rows {
        match &mut row.kind {
            RowKind::Card { card_id, card } => {
                cards.insert(card_id.to_string(), card.clone());
            }
            RowKind::User { text, .. } => {
                let Some(action) = CardAction::parse_wire(text) else {
                    continue;
                };
                let label = cards
                    .get(&action.card_id)
                    .and_then(|card| selected_label(card, &action));
                let acknowledgement = label
                    .map(|label| format!("Selected: {label}"))
                    .unwrap_or_else(|| "Card selection sent.".into());
                row.version = fnv1a(acknowledgement.as_bytes()) << 1 | (row.version & 1);
                *text = acknowledgement.into();
                row.copy_text = Some(text.clone());
            }
            _ => {}
        }
    }
}

fn children(list: &ChildList, state: &CardState, scope: &Scope) -> Vec<(String, Scope)> {
    match list {
        ChildList::Static(ids) => ids.iter().map(|id| (id.clone(), scope.clone())).collect(),
        ChildList::Template { component_id, path } => {
            let path = scope.absolute(path);
            state
                .data
                .get(&path)
                .and_then(|value| value.as_array())
                .map(|items| {
                    (0..items.len().min(256))
                        .map(|ix| (component_id.clone(), Scope::item(format!("{path}/{ix}"))))
                        .collect()
                })
                .unwrap_or_default()
        }
    }
}

fn descendants(kind: &ComponentKind, state: &CardState, scope: &Scope) -> Vec<(String, Scope)> {
    match kind {
        ComponentKind::Row { children: list, .. }
        | ComponentKind::Column { children: list, .. }
        | ComponentKind::List { children: list, .. } => children(list, state, scope),
        ComponentKind::Card { child } | ComponentKind::Button { child, .. } => {
            vec![(child.clone(), scope.clone())]
        }
        ComponentKind::Tabs { tabs } => tabs
            .iter()
            .map(|tab| (tab.child.clone(), scope.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

fn button_text(card: &Card, state: &CardState, id: &str, scope: &Scope) -> Option<String> {
    let mut pending = vec![(id.to_string(), scope.clone())];
    let mut labels = Vec::new();
    for _ in 0..64 {
        let Some((id, scope)) = pending.pop() else {
            break;
        };
        let Some(component) = card.get(&id) else {
            continue;
        };
        match &component.kind {
            ComponentKind::Text { text, .. } => {
                let label = resolve_string(&state.data, &scope, text);
                if !label.trim().is_empty() {
                    labels.push(label.trim().to_string());
                }
            }
            other => pending.extend(descendants(other, state, &scope).into_iter().rev()),
        }
    }
    (!labels.is_empty()).then(|| labels.join(" "))
}

fn selected_label(card: &Card, selected: &CardAction) -> Option<String> {
    let state = CardState::new(card);
    let mut pending = vec![("root".to_string(), Scope::root())];
    let mut matches = Vec::new();
    for _ in 0..256 {
        let Some((id, scope)) = pending.pop() else {
            break;
        };
        let Some(component) = card.get(&id) else {
            continue;
        };
        if let ComponentKind::Button { child, action, .. } = &component.kind
            && let Some(CardEvent::Action(candidate)) = state.resolve_action(card, &scope, action)
            && candidate == *selected
            && let Some(label) = button_text(card, &state, child, &scope)
        {
            matches.push(label);
        }
        pending.extend(
            descendants(&component.kind, &state, &scope)
                .into_iter()
                .rev(),
        );
    }
    matches.sort();
    matches.dedup();
    (matches.len() == 1).then(|| matches.remove(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn card() -> Card {
        let mut card = surya_a2ui::parse_card(&json!({"components": [
            {"id":"root","component":"Column","children":["apples","pears"]},
            {"id":"apples","component":"Button","child":"a","action":{"event":{"name":"choose","context":{"fruit":"apples"}}}},
            {"id":"a","component":"Text","text":"Choose apples"},
            {"id":"pears","component":"Button","child":"p","action":{"event":{"name":"choose","context":{"fruit":"pears"}}}},
            {"id":"p","component":"Text","text":"Choose pears"}
        ]}));
        card.card_id = "private-id".into();
        card
    }

    #[test]
    fn same_action_name_uses_payload_to_find_the_clicked_button_label() {
        let card = card();
        for fruit in ["apples", "pears"] {
            let action = CardAction {
                card_id: "private-id".into(),
                action: "choose".into(),
                payload: json!({"fruit": fruit}),
            };
            assert_eq!(
                selected_label(&card, &action),
                Some(format!("Choose {fruit}"))
            );
            assert_eq!(CardAction::parse_wire(&action.to_wire()), Some(action));
        }
    }

    #[test]
    fn unknown_or_ambiguous_actions_do_not_guess_a_selection() {
        let card = card();
        let action = CardAction {
            card_id: "private-id".into(),
            action: "choose".into(),
            payload: json!({"fruit":"oranges"}),
        };
        assert_eq!(selected_label(&card, &action), None);
    }
    fn row(kind: RowKind) -> Row {
        Row {
            id: "row".into(),
            version: 1,
            turn_start: true,
            kind,
            entry_id: "entry".into(),
            timestamp: None,
            copy_text: None,
            stopped: false,
            retry_prompt: None,
        }
    }

    #[test]
    fn transcript_and_copy_show_the_label_without_action_id_or_json() {
        let card = Arc::new(card());
        let wire = CardAction {
            card_id: "private-id".into(),
            action: "choose".into(),
            payload: json!({"fruit":"apples"}),
        }
        .to_wire();
        let user = || {
            row(RowKind::User {
                text: wire.clone().into(),
                mentions: Arc::default(),
                attachments: Arc::default(),
                badges: Arc::default(),
                pending: true,
            })
        };
        let mut rows = vec![
            row(RowKind::Card {
                card_id: "private-id".into(),
                card,
            }),
            user(),
        ];
        project(&mut rows);
        let RowKind::User { text, pending, .. } = &rows[1].kind else {
            panic!("user row")
        };
        assert_eq!(text.as_ref(), "Selected: Choose apples");
        assert!(*pending);
        assert_eq!(
            rows[1].copy_text.as_deref(),
            Some("Selected: Choose apples")
        );
        // A partial history still hides the transport; never guess a label.
        let mut without_card = vec![user()];
        project(&mut without_card);
        assert_eq!(
            without_card[0].copy_text.as_deref(),
            Some("Card selection sent.")
        );
        assert!(
            wire.contains("private-id"),
            "stored protocol was not mutated"
        );
    }
}
