//! The approval-policy page: every always-allow rule on this device, with a
//! way to take one back.
//!
//! Decision 20 gives Settings this page because a rule made from a card is
//! the only permission the user does not see again — so the one place they
//! can review and revoke has to exist and has to be plain.

use gpui::{Context, Entity, Render, SharedString, Task, Window, div, prelude::*, px};
use zeron_proto::{AllowRule, RuleScope};
use zeron_rpc::methods;

use crate::inbox::chrome::{ButtonTone, body_text, button, command_text, empty_state, row_card};
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
                    .unwrap_or("this workspace");
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
            "Bash, in this workspace"
        );
    }
}
