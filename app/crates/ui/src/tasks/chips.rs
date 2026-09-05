//! Small presentational pieces of a task card: the count chip, the owner
//! chip, link chips, and the text helpers behind them. No state, no RPC.

use gpui::{AnyElement, IntoElement, SharedString, div, prelude::*, px};

use crate::theme::{Theme, hairline, ink};
use crate::typography::ui_rems;

pub fn count_chip(theme: &Theme, count: usize) -> gpui::Div {
    div()
        .px(px(7.0))
        .py(px(1.0))
        .rounded(px(999.0))
        .bg(ink(0.08)) // TOKEN: surya.chip_bg
        .text_size(ui_rems(11.0))
        .text_color(theme.text_muted)
        .child(SharedString::from(count.to_string()))
}

/// "you" for the user (or nobody yet), else the agent id with a dot.
pub fn owner_label(owner: Option<&str>) -> (&str, bool) {
    match owner.map(str::trim) {
        None | Some("") | Some("user") | Some("you") => ("you", false),
        Some(agent) => (agent, true),
    }
}

pub fn owner_chip(theme: &Theme, owner: Option<&str>) -> AnyElement {
    let (label, is_agent) = owner_label(owner);
    chip(theme)
        .children(is_agent.then(|| {
            div()
                .w(px(6.0))
                .h(px(6.0))
                .rounded(px(3.0))
                .bg(theme.accent) // TOKEN: surya.agent_dot
        }))
        .child(SharedString::from(label.to_string()))
        .into_any_element()
}

pub fn link_chip(theme: &Theme, link: &str) -> AnyElement {
    chip(theme)
        .font_family(theme.font_mono.clone())
        .child(SharedString::from(link_label(link)))
        .into_any_element()
}

fn chip(theme: &Theme) -> gpui::Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(5.0))
        .px(px(7.0))
        .py(px(2.0))
        .rounded(px(999.0))
        .border_1()
        .border_color(hairline(0.10))
        .bg(ink(0.03))
        .text_size(ui_rems(11.5))
        .text_color(theme.text_muted)
}

/// `t-1` stays; a UUID shows its first 6 characters.
pub fn short_id(id: &str) -> String {
    if id.len() <= 8 {
        id.to_string()
    } else {
        id.chars().take(6).collect()
    }
}

/// `https://github.com/o/r/pull/12` → `o/r#12`; other URLs show their host.
pub fn link_label(link: &str) -> String {
    let stripped = link
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let parts: Vec<&str> = stripped.split('/').filter(|p| !p.is_empty()).collect();
    match parts.as_slice() {
        ["github.com", owner, repo, kind, number, ..] if *kind == "pull" || *kind == "issues" => {
            format!("{owner}/{repo}#{number}")
        }
        [host, ..] => (*host).to_string(),
        [] => link.to_string(),
    }
}

/// First `max` characters of the first line, with an ellipsis when cut.
pub fn preview(text: &str, max: usize) -> String {
    let line = text.lines().next().unwrap_or("");
    if line.chars().count() <= max {
        line.to_string()
    } else {
        let cut: String = line.chars().take(max).collect();
        format!("{}…", cut.trim_end())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_and_previews() {
        assert_eq!(owner_label(None), ("you", false));
        assert_eq!(owner_label(Some("user")), ("you", false));
        assert_eq!(owner_label(Some("raven")), ("raven", true));
        assert_eq!(short_id("t-1"), "t-1");
        assert_eq!(short_id("3f2a9c1e-0000-4000-8000-000000000000"), "3f2a9c");
        assert_eq!(
            link_label("https://github.com/screamyx/surya/pull/6"),
            "screamyx/surya#6"
        );
        assert_eq!(link_label("https://example.com/x/y"), "example.com");
        assert_eq!(preview("one line\nsecond", 90), "one line");
        assert_eq!(preview("abcdefghij", 4), "abcd…");
    }
}
