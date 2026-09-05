//! Per-card element budget. Depth alone does not bound a tree: a template
//! whose item component is itself fans out 200^n. Every rendered node takes
//! one unit; past the cap the renderer stops and the card shows the
//! "could not be drawn" panel. [`count_nodes`] is the same walk without
//! elements, for tests and for a cheap pre-check.

use std::cell::Cell;

use serde_json::Value;

use crate::data::Scope;
use crate::model::{Card, ChildList, ComponentKind};
use crate::state::CardState;

/// Elements a single card may create per render.
pub const MAX_NODES: usize = 5_000;

#[derive(Debug)]
pub struct Budget {
    cap: usize,
    used: Cell<usize>,
    exceeded: Cell<bool>,
}

impl Default for Budget {
    fn default() -> Self {
        Self::new(MAX_NODES)
    }
}

impl Budget {
    pub fn new(cap: usize) -> Self {
        Self {
            cap,
            used: Cell::new(0),
            exceeded: Cell::new(false),
        }
    }

    /// Take one unit; `false` once the cap is spent (and stays false).
    pub fn take(&self) -> bool {
        let used = self.used.get();
        if used >= self.cap {
            self.exceeded.set(true);
            return false;
        }
        self.used.set(used + 1);
        true
    }

    pub fn used(&self) -> usize {
        self.used.get()
    }

    pub fn cap(&self) -> usize {
        self.cap
    }

    pub fn exceeded(&self) -> bool {
        self.exceeded.get()
    }
}

/// The child ids a container instantiates, with each child's scope. Shared
/// by the renderer and the counter so both see the same fan-out.
pub fn expand_children(
    card: &Card,
    state: &CardState,
    scope: &Scope,
    list: &ChildList,
    max_items: usize,
) -> Vec<(String, Scope)> {
    let _ = card;
    match list {
        ChildList::Static(ids) => ids.iter().map(|id| (id.clone(), scope.clone())).collect(),
        ChildList::Template { component_id, path } => {
            let abs = scope.absolute(path);
            let count = state
                .data
                .get(&abs)
                .and_then(Value::as_array)
                .map_or(0, Vec::len)
                .min(max_items);
            (0..count)
                .map(|ix| (component_id.clone(), Scope::item(format!("{abs}/{ix}"))))
                .collect()
        }
    }
}

/// Walk the tree the way the renderer does and count nodes against
/// `budget`. Returns the count; `budget.exceeded()` says whether the cap
/// was hit. Never recurses past `max_depth`.
pub fn count_nodes(
    card: &Card,
    state: &CardState,
    budget: &Budget,
    max_depth: usize,
    max_items: usize,
) -> usize {
    fn walk(
        card: &Card,
        state: &CardState,
        budget: &Budget,
        id: &str,
        scope: &Scope,
        depth: usize,
        max_depth: usize,
        max_items: usize,
    ) {
        if depth > max_depth || !budget.take() {
            return;
        }
        let Some(c) = card.get(id) else {
            return;
        };
        let children: Vec<(String, Scope)> = match &c.kind {
            ComponentKind::Row { children, .. }
            | ComponentKind::Column { children, .. }
            | ComponentKind::List { children, .. } => {
                expand_children(card, state, scope, children, max_items)
            }
            ComponentKind::Card { child } | ComponentKind::Button { child, .. } => {
                vec![(child.clone(), scope.clone())]
            }
            ComponentKind::Tabs { tabs } => tabs
                .get(state.selected_tab(&c.id).min(tabs.len().saturating_sub(1)))
                .map(|t| vec![(t.child.clone(), scope.clone())])
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        for (child, child_scope) in children {
            if budget.exceeded() {
                return;
            }
            walk(card, state, budget, &child, &child_scope, depth + 1, max_depth, max_items);
        }
    }
    walk(card, state, budget, crate::model::ROOT_ID, &Scope::root(), 0, max_depth, max_items);
    budget.used()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_card;
    use serde_json::json;

    #[test]
    fn budget_counts_and_stops() {
        let b = Budget::new(3);
        assert!(b.take() && b.take() && b.take());
        assert!(!b.take());
        assert!(b.exceeded());
        assert_eq!((b.used(), b.cap()), (3, 3));
    }

    /// A template whose item is the template's own container: 3 items per
    /// level, unbounded depth. The walk stops at the budget, not at 3^24.
    #[test]
    fn self_referencing_template_hits_the_budget_not_the_stack() {
        let card = parse_card(&json!({
            "components": [
                {"id": "root", "component": "Column", "children": {"componentId": "root", "path": "/items"}}
            ],
            "data": {"items": [1, 2, 3]}
        }));
        assert!(card.errors.is_empty());
        let state = CardState::new(&card);
        let budget = Budget::new(500);
        let used = count_nodes(&card, &state, &budget, 24, 200);
        assert!(budget.exceeded(), "used {used}");
        assert_eq!(used, 500);
        // Relative template paths resolve per item, so the nesting is real:
        // `/items` is absolute here, which is exactly the runaway shape.
        let small = Budget::new(5_000);
        let card2 = parse_card(&json!({
            "components": [
                {"id": "root", "component": "Column", "children": ["a", "b"]},
                {"id": "a", "component": "Text", "text": "x"},
                {"id": "b", "component": "Row", "children": {"componentId": "a", "path": "/items"}}
            ],
            "data": {"items": [1, 2, 3]}
        }));
        assert_eq!(count_nodes(&card2, &CardState::new(&card2), &small, 24, 200), 6);
        assert!(!small.exceeded());
    }
}
