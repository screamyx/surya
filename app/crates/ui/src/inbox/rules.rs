//! The approval-policy page: every always-allow rule on this device, with a
//! way to take one back.
//!
//! Decision 20 gives Settings this page because a rule made from a card is
//! the only permission the user does not see again — so the one place they
//! can review and revoke has to exist and has to be plain.

use gpui::{Context, Entity, Render, SharedString, Task, Window, div, prelude::*, px};
use surya_proto::{AllowRule, RuleScope};
use surya_rpc::methods;

use crate::inbox::chrome::{ButtonTone, body_text, button, command_text, empty_state, row_card};
use crate::inbox::model::AlwaysAllowScope;
use crate::state::AppState;
use crate::theme::Theme;
use crate::typography::ui_rems;

pub struct RulesPane {
    /// `None` in the demo and in tests.
    state: Option<Entity<AppState>>,
    rules: Vec<AllowRule>,
    failure: Option<SharedString>,
    _refresh: Option<Task<()>>,
}

impl RulesPane {
    pub fn new(state: Entity<AppState>, cx: &mut Context<Self>) -> Self {
        let mut pane = Self {
            state: Some(state),
            rules: Vec::new(),
            failure: None,
            _refresh: None,
        };
        pane.reload(cx);
        pane
    }

    /// Fixtures instead of an engine — the demo entry and the tests.
    pub fn demo(rules: Vec<AllowRule>) -> Self {
        Self {
            state: None,
            rules,
            failure: None,
            _refresh: None,
        }
    }

    pub fn set_rules(&mut self, rules: Vec<AllowRule>) {
        self.rules = rules;
    }

    pub fn rules(&self) -> &[AllowRule] {
        &self.rules
    }

    /// `ListAllowRules` is a plain call, not a stream: the table only changes
    /// when someone on this device answers a card or deletes a row, and both
    /// paths come back through here.
    pub fn reload(&mut self, cx: &mut Context<Self>) {
        let Some(engine) = self
            .state
            .as_ref()
            .and_then(|state| state.read(cx).engine().cloned())
        else {
            return;
        };
        self._refresh = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(methods::LIST_ALLOW_RULES, serde_json::json!({}))
                .await;
            this.update(cx, |pane, cx| {
                match result {
                    Ok(value) => match serde_json::from_value::<Vec<AllowRule>>(value) {
                        Ok(rules) => {
                            pane.rules = rules;
                            pane.failure = None;
                        }
                        Err(err) => {
                            pane.failure = Some(format!("Could not read the rules: {err}").into())
                        }
                    },
                    Err(err) => pane.failure = Some(format!("Could not load rules: {err}").into()),
                }
                cx.notify();
            })
            .ok();
        }));
    }

    fn delete(&mut self, rule_id: String, cx: &mut Context<Self>) {
        let Some(engine) = self
            .state
            .as_ref()
            .and_then(|state| state.read(cx).engine().cloned())
        else {
            return;
        };
        // Drop it locally first so the row goes at once; a failed call
        // reloads the truth back over the top.
        self.rules.retain(|rule| rule.id != rule_id);
        self.failure = None;
        cx.notify();
        self._refresh = Some(cx.spawn(async move |this, cx| {
            let result = engine
                .client()
                .call(
                    methods::DELETE_ALLOW_RULE,
                    serde_json::json!({ "ruleId": rule_id }),
                )
                .await;
            this.update(cx, |pane, cx| {
                if let Err(err) = result {
                    pane.failure = Some(format!("Could not delete the rule: {err}").into());
                }
                pane.reload(cx);
            })
            .ok();
        }));
    }

    /// "Bash, in project-jag" / "Bash, everywhere" — where a rule reaches,
    /// said in words rather than as a path the user has to parse.
    fn scope_line(rule: &AllowRule) -> String {
        match rule.scope {
            RuleScope::Global => format!("{}, everywhere", rule.tool_name),
            RuleScope::Workspace => {
                let folder = rule
                    .workspace_path
                    .as_deref()
                    .map(|path| path.trim_end_matches('/'))
                    .and_then(|path| path.rsplit('/').next())
                    .filter(|name| !name.is_empty())
                    // Falls back to the SAME words the permission card and
                    // the Needs you chip use for this reach, taken from the
                    // one place that owns them. Written out separately it
                    // said "this workspace" while both other surfaces said
                    // "this project" (PERM-03 fixed those two and missed
                    // this one).
                    .unwrap_or_else(|| AlwaysAllowScope::ThisWorkspace.label());
                format!("{}, in {folder}", rule.tool_name)
            }
        }
    }

    fn render_rule(&self, rule: &AllowRule, cx: &mut Context<Self>) -> gpui::AnyElement {
        let theme = Theme::of(cx);
        let rule_id = rule.id.clone();
        row_card(theme)
            .id(SharedString::from(format!("rule-{}", rule.id)))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .whitespace_nowrap()
                            .text_size(ui_rems(13.0))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(theme.text)
                            .child(SharedString::from(rule.name.clone())),
                    )
                    .child(
                        button(theme, ButtonTone::Danger, "Delete")
                            .id(SharedString::from(format!("delete-rule-{}", rule.id)))
                            .on_click(cx.listener(move |pane, _, _, cx| {
                                pane.delete(rule_id.clone(), cx);
                            })),
                    ),
            )
            .child(body_text(theme, Self::scope_line(rule)))
            .child(command_text(theme, rule.pattern.clone()))
            .into_any_element()
    }
}

impl Render for RulesPane {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rules = self.rules.clone();
        let cards: Vec<gpui::AnyElement> =
            rules.iter().map(|rule| self.render_rule(rule, cx)).collect();
        let empty = cards.is_empty();
        let theme = Theme::of(cx);
        div()
            .id("rules-pane")
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .p(px(12.0))
            .when_some(self.failure.clone(), |el, failure| {
                el.child(
                    div()
                        .text_size(ui_rems(12.0))
                        .text_color(theme.danger)
                        .child(failure),
                )
            })
            .when(empty, |el| {
                el.child(empty_state(
                    theme,
                    "No always-allow rules. Every tool asks first.",
                ))
            })
            .children(cards)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn rule(scope: RuleScope, workspace: Option<&str>) -> AllowRule {
        AllowRule {
            id: "r-1".into(),
            name: "Bash php artisan migrate* in project-jag".into(),
            scope,
            workspace_path: workspace.map(str::to_string),
            tool_name: "Bash".into(),
            pattern: "php artisan migrate*".into(),
            exact: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn the_scope_line_names_the_folder_not_the_path() {
        assert_eq!(
            RulesPane::scope_line(&rule(RuleScope::Workspace, Some("/repos/project-jag"))),
            "Bash, in project-jag"
        );
        assert_eq!(
            RulesPane::scope_line(&rule(RuleScope::Workspace, Some("/repos/project-jag/"))),
            "Bash, in project-jag"
        );
        assert_eq!(
            RulesPane::scope_line(&rule(RuleScope::Global, None)),
            "Bash, everywhere"
        );
        // A workspace rule that somehow lost its path still reads sensibly
        // rather than showing an empty gap.
        assert_eq!(
            RulesPane::scope_line(&rule(RuleScope::Workspace, None)),
            "Bash, in this project"
        );
    }

    /// The fallback is the third surface to name this reach, and it was the
    /// one PERM-03 missed: the card and the Needs you chip were changed to
    /// "project" and this still said "workspace".
    ///
    /// Pinned to the label rather than to a literal, so the three cannot
    /// drift apart again without a failure here.
    #[test]
    fn the_pathless_fallback_uses_the_same_words_as_the_card_and_the_chip() {
        let shared = AlwaysAllowScope::ThisWorkspace.label();
        assert_eq!(
            RulesPane::scope_line(&rule(RuleScope::Workspace, None)),
            format!("Bash, in {shared}")
        );
        assert!(
            !RulesPane::scope_line(&rule(RuleScope::Workspace, None)).contains("workspace"),
            "workspace is the engine's word for this scope, never the user's"
        );
    }

    /// The fix is real and unphotographable at the same time unless a demo
    /// row reaches it. `--inbox-demo` is the only thing that builds this
    /// page (`demo_run.rs` calls `RulesPane::demo(demo::rules())`, and
    /// nothing else constructs one), so the fixture is the whole rig.
    ///
    /// The two original rows yield "in orchard" and "everywhere". Drop the
    /// third and this fails, which is the point: the screenshot the PR ships
    /// would show two rows that were never broken.
    #[test]
    fn the_demo_fixture_renders_the_pathless_fallback() {
        let lines: Vec<String> = crate::inbox::demo::rules()
            .iter()
            .map(RulesPane::scope_line)
            .collect();
        let wanted = format!("in {}", AlwaysAllowScope::ThisWorkspace.label());
        assert!(
            lines.iter().any(|line| line.ends_with(&wanted)),
            "no --inbox-demo row reaches the pathless fallback; the rules page shows {lines:?}"
        );
    }
}
