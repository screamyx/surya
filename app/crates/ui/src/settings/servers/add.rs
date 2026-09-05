//! The Add server dialog: four text fields, the parser that turns them into a
//! [`ServerEntry`], and the modal that shows them. The page owns the open
//! dialog and the list; this module only builds and renders.

use gpui::{
    AnyElement, Context, Entity, Focusable, SharedString, Subscription, Window, div, prelude::*, px,
};

use super::ServersPage;
use crate::composer::{ComposerInput, ComposerInputEvent};
use crate::popover;
use crate::settings::ServerEntry;
use crate::theme::Theme;

/// Build a server row from the dialog's raw text. Pure so the parsing rules
/// are testable: `host` may carry `ws://` and `:port`; the port field wins
/// when both are given; an empty name falls back to the host.
pub fn parse_server(
    name: &str,
    host: &str,
    port: &str,
    token: &str,
) -> Result<ServerEntry, String> {
    let mut host = host.trim();
    for prefix in ["ws://", "wss://", "http://", "https://"] {
        host = host.strip_prefix(prefix).unwrap_or(host);
    }
    let host = host.trim_end_matches('/');
    let (host, inline_port) = match host.rsplit_once(':') {
        Some((h, p))
            if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && !h.contains(':') =>
        {
            (h, Some(p))
        }
        _ => (host, None),
    };
    if host.is_empty() {
        return Err("Host is required (an IP address or a name your network resolves).".into());
    }
    let port = match port.trim() {
        "" => inline_port.unwrap_or("27654"),
        given => given,
    };
    let port: u16 = port
        .parse::<u16>()
        .ok()
        .filter(|p| *p > 0)
        .ok_or_else(|| format!("Port {port:?} is not a number between 1 and 65535."))?;
    let name = match name.trim() {
        "" => host.to_string(),
        given => given.to_string(),
    };
    let token = match token.trim() {
        "" => None,
        given => Some(given.to_string()),
    };
    Ok(ServerEntry {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        host: host.to_string(),
        port,
        token,
    })
}

/// The four fields of the Add server dialog plus the last rejection.
pub(super) struct AddDialog {
    pub(super) name: Entity<ComposerInput>,
    pub(super) host: Entity<ComposerInput>,
    pub(super) port: Entity<ComposerInput>,
    pub(super) token: Entity<ComposerInput>,
    pub(super) error: Option<SharedString>,
    _events: Vec<Subscription>,
}

impl AddDialog {
    pub(super) fn open(window: &mut Window, cx: &mut Context<ServersPage>) -> AddDialog {
        let mut events = Vec::with_capacity(4);
        let mut field = |placeholder: &'static str, cx: &mut Context<ServersPage>| {
            let input = cx.new(|cx| ComposerInput::new(placeholder, cx));
            events.push(
                cx.subscribe(&input, |this: &mut ServersPage, _, event, cx| {
                    if matches!(event, ComposerInputEvent::Submitted) {
                        this.submit_add(cx);
                    }
                }),
            );
            input
        };
        let name = field("Name (optional)", cx);
        let host = field("Host, e.g. 100.64.0.9 or build-box", cx);
        let port = field("Port (default 27654)", cx);
        let token = field("Token from `zeron status` on that machine", cx);
        window.focus(&host.focus_handle(cx), cx);
        AddDialog {
            name,
            host,
            port,
            token,
            error: None,
            _events: events,
        }
    }
}

pub(super) fn render(
    dialog: &AddDialog,
    viewport: gpui::Size<gpui::Pixels>,
    cx: &mut Context<ServersPage>,
) -> AnyElement {
    use crate::settings::widgets;
    let theme = Theme::of(cx).clone();
    let field = |label: &str, input: Entity<ComposerInput>| {
        div()
            .mt(px(12.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(widgets::field_label(&theme, label.to_string()))
            .child(popover::dialog_field(input.into_any_element()))
    };
    let card = popover::dialog_card(&theme)
        .child(popover::dialog_title(&theme, "Add server"))
        .child(div().mt(px(6.0)).child(popover::dialog_body(
            &theme,
            "The other machine runs `zeron headless --bind <its address>`. \
                 `zeron status` there prints the token.",
        )))
        .child(field("Name", dialog.name.clone()))
        .child(field("Host", dialog.host.clone()))
        .child(field("Port", dialog.port.clone()))
        .child(field("Token", dialog.token.clone()))
        .when_some(dialog.error.clone(), |el, message| {
            el.child(
                div()
                    .mt(px(12.0))
                    .child(widgets::error_strip(&theme, message)),
            )
        })
        .child(
            div()
                .mt(px(16.0))
                .flex()
                .flex_row()
                .justify_end()
                .gap(px(8.0))
                .child(
                    popover::btn_ghost(&theme, "Cancel", "add-server-cancel")
                        .id("add-server-cancel")
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.add = None;
                            cx.notify();
                        })),
                )
                .child(
                    popover::btn_primary(&theme, "Add")
                        .id("add-server-save")
                        .on_click(cx.listener(|this, _, _, cx| this.submit_add(cx))),
                ),
        )
        .into_any_element();
    popover::modal("add-server-dialog", viewport, card)
}

/// `remove_id` is the saved entry the row's Remove button drops (`None`

#[cfg(test)]
mod tests {
    use super::parse_server;

    /// Save on the Command line row goes through `parse_server` with the
    /// dialed url as the host field: the saved entry must land at the same
    /// address the app dialed (asked=2 same=2).
    #[test]
    fn a_dialed_url_saves_back_to_the_same_address() {
        for dialed in ["ws://pc-ajim:27700", "ws://100.83.77.3:27700/"] {
            let entry = parse_server("", dialed, "", "tok").unwrap();
            assert_eq!(
                crate::settings::servers::canonical_url(&entry.url()),
                crate::settings::servers::canonical_url(dialed),
                "{dialed}"
            );
            assert_eq!(entry.token.as_deref(), Some("tok"));
        }
    }

    #[test]
    fn host_and_port_parse_from_plain_fields() {
        let entry = parse_server("Build box", "100.64.0.9", "27654", "abc").unwrap();
        assert_eq!(entry.name, "Build box");
        assert_eq!(entry.host, "100.64.0.9");
        assert_eq!(entry.port, 27654);
        assert_eq!(entry.token.as_deref(), Some("abc"));
        assert_eq!(entry.url(), "ws://100.64.0.9:27654");
    }

    #[test]
    fn host_may_carry_scheme_and_port() {
        let entry = parse_server("", "ws://build-box:27700/", "", "").unwrap();
        assert_eq!(entry.name, "build-box");
        assert_eq!(entry.host, "build-box");
        assert_eq!(entry.port, 27700);
        assert_eq!(entry.token, None);
    }

    #[test]
    fn port_field_wins_and_defaults() {
        assert_eq!(parse_server("", "h:1", "2", "").unwrap().port, 2);
        assert_eq!(parse_server("", "h", "", "").unwrap().port, 27654);
    }

    #[test]
    fn bad_input_is_rejected() {
        assert!(parse_server("", "", "1", "").is_err());
        assert!(parse_server("", "h", "0", "").is_err());
        assert!(parse_server("", "h", "70000", "").is_err());
        assert!(parse_server("", "h", "abc", "").is_err());
    }
}
