//! Settings → Servers: the remote engines this app can drive directly over
//! the network (decision 18: LAN, VPN or tailnet, no cloud in between). Each
//! row is a `zeron headless --bind <addr>` on another machine; its token comes
//! from `zeron status` there. Connect swaps the engine the whole app talks
//! to. The shell owns the swap and persists the list, so this page only emits.

use gpui::{
    AnyElement, Context, Entity, EventEmitter, Focusable, SharedString, Subscription, Window,
    div, prelude::*, px,
};
use crate::composer::{ComposerInput, ComposerInputEvent};
use crate::popover;
use crate::settings::ServerEntry;
use crate::state::{AppState, ConnectionStatus, EngineMode, RemoteEngineTarget};
use crate::theme::Theme;

pub enum ServersEvent {
    /// The list or the active choice changed; the shell persists it.
    Changed {
        servers: Vec<ServerEntry>,
        active: Option<String>,
    },
    /// Dial this engine now (`None` = back to the local engine).
    Connect(Option<RemoteEngineTarget>),
}

/// Build a server row from the dialog's raw text. Pure so the parsing rules
/// are testable: `host` may carry `ws://` and `:port`; the port field wins
/// when both are given; an empty name falls back to the host.
pub fn parse_server(name: &str, host: &str, port: &str, token: &str) -> Result<ServerEntry, String> {
    let mut host = host.trim();
    for prefix in ["ws://", "wss://", "http://", "https://"] {
        host = host.strip_prefix(prefix).unwrap_or(host);
    }
    let host = host.trim_end_matches('/');
    let (host, inline_port) = match host.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) && !h.contains(':') => {
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

struct AddDialog {
    name: Entity<ComposerInput>,
    host: Entity<ComposerInput>,
    port: Entity<ComposerInput>,
    token: Entity<ComposerInput>,
    error: Option<SharedString>,
    _events: Vec<Subscription>,
}

pub struct ServersPage {
    state: Entity<AppState>,
    servers: Vec<ServerEntry>,
    active: Option<String>,
    add: Option<AddDialog>,
    _observe: Subscription,
}

impl EventEmitter<ServersEvent> for ServersPage {}

impl ServersPage {
    pub fn new(
        state: Entity<AppState>,
        servers: Vec<ServerEntry>,
        active: Option<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        let observe = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            servers,
            active,
            add: None,
            _observe: observe,
        }
    }

    fn emit_changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(ServersEvent::Changed {
            servers: self.servers.clone(),
            active: self.active.clone(),
        });
    }

    fn connect(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        let target = match &id {
            Some(id) => match self.servers.iter().find(|s| &s.id == id) {
                Some(entry) => Some(entry.target()),
                None => return,
            },
            None => None,
        };
        self.active = id;
        self.emit_changed(cx);
        cx.emit(ServersEvent::Connect(target));
        cx.notify();
    }

    fn remove(&mut self, id: String, cx: &mut Context<Self>) {
        self.servers.retain(|s| s.id != id);
        let was_active = self.active.as_deref() == Some(id.as_str());
        if was_active {
            self.active = None;
        }
        self.emit_changed(cx);
        if was_active {
            cx.emit(ServersEvent::Connect(None));
        }
        cx.notify();
    }

    fn open_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut events = Vec::with_capacity(4);
        let mut field = |placeholder: &'static str, cx: &mut Context<Self>| {
            let input = cx.new(|cx| ComposerInput::new(placeholder, cx));
            events.push(cx.subscribe(&input, |this: &mut Self, _, event, cx| {
                if matches!(event, ComposerInputEvent::Submitted) {
                    this.submit_add(cx);
                }
            }));
            input
        };
        let name = field("Name (optional)", cx);
        let host = field("Host, e.g. 100.64.0.9 or build-box", cx);
        let port = field("Port (default 27654)", cx);
        let token = field("Token from `zeron status` on that machine", cx);
        window.focus(&host.focus_handle(cx), cx);
        self.add = Some(AddDialog {
            name,
            host,
            port,
            token,
            error: None,
            _events: events,
        });
        cx.notify();
    }

    fn submit_add(&mut self, cx: &mut Context<Self>) {
        let Some(dialog) = self.add.as_ref() else {
            return;
        };
        let parsed = parse_server(
            dialog.name.read(cx).text(),
            dialog.host.read(cx).text(),
            dialog.port.read(cx).text(),
            dialog.token.read(cx).text(),
        );
        match parsed {
            Ok(entry) => {
                self.add = None;
                self.servers.push(entry);
                self.emit_changed(cx);
            }
            Err(message) => {
                if let Some(dialog) = self.add.as_mut() {
                    dialog.error = Some(message.into());
                }
            }
        }
        cx.notify();
    }

    /// One line of truth about what the app is talking to right now.
    fn status_line(&self, cx: &Context<Self>) -> (SharedString, bool) {
        let state = self.state.read(cx);
        let mode = state.engine().map(|engine| engine.mode());
        match (&state.connection, mode) {
            (ConnectionStatus::Connecting, _) => ("Connecting…".into(), false),
            (ConnectionStatus::Failed(message), _) => (format!("Not connected: {message}").into(), true),
            (ConnectionStatus::Ready, Some(EngineMode::Remote { url })) => {
                let name = self
                    .active
                    .as_deref()
                    .and_then(|id| self.servers.iter().find(|s| s.id == id))
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| url.clone());
                (format!("Connected to {name} ({url})").into(), false)
            }
            (ConnectionStatus::Ready, _) => ("Using the engine on this computer".into(), false),
        }
    }

    fn render_add_dialog(
        &mut self,
        viewport: gpui::Size<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        use crate::settings::widgets;
        let theme = Theme::of(cx).clone();
        let dialog = self.add.as_ref()?;
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
                el.child(div().mt(px(12.0)).child(widgets::error_strip(&theme, message)))
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
        Some(popover::modal("add-server-dialog", viewport, card))
    }

    fn render_row(
        &self,
        ix: usize,
        title: String,
        meta: Vec<String>,
        icon_path: &'static str,
        id: Option<String>,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::settings::widgets;
        let is_active = self.active == id;
        let meta: Vec<AnyElement> = meta
            .into_iter()
            .map(|text| div().child(SharedString::from(text)).into_any_element())
            .collect();
        let connect_id = id.clone();
        let mut row = widgets::card_row(theme, ix == 0)
            .child(widgets::row_tile(theme, icon_path))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(widgets::row_title(theme, title))
                    .child(widgets::meta_line(theme, meta)),
            );
        row = if is_active {
            row.child(widgets::badge_active(theme, "Active"))
        } else {
            row.child(
                widgets::ghost_action(theme)
                    .id(("server-connect", ix))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.connect(connect_id.clone(), cx);
                    }))
                    .child(
                        crate::icons::icon(crate::icons::GLOBAL)
                            .size(px(14.0))
                            .text_color(theme.text_muted),
                    )
                    .child(SharedString::from("Connect")),
            )
        };
        if let Some(remove_id) = id {
            row = row.child(
                widgets::ghost_action(theme)
                    .id(("server-remove", ix))
                    .opacity(0.7)
                    .hover(|s| s.opacity(1.0).bg(crate::theme::ink(0.06)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.remove(remove_id.clone(), cx);
                    }))
                    .child(
                        crate::icons::icon(crate::icons::TRASH_BIN_MINIMALISTIC)
                            .size(px(14.0))
                            .text_color(theme.text_muted),
                    )
                    .child(SharedString::from("Remove")),
            );
        }
        row.into_any_element()
    }
}

impl Render for ServersPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use crate::settings::widgets;
        let theme = Theme::of(cx).clone();
        let (status, status_is_error) = self.status_line(cx);
        let dialog = self.render_add_dialog(window.viewport_size(), cx);
        let count = self.servers.len();

        let mut rows: Vec<AnyElement> = vec![self.render_row(
            0,
            "This computer".into(),
            vec!["Engine started by this app, or a daemon on the local port".into()],
            crate::icons::MONITOR,
            None,
            &theme,
            cx,
        )];
        for (ix, server) in self.servers.clone().into_iter().enumerate() {
            let meta = vec![
                format!("{}:{}", server.host, server.port),
                if server.token.is_some() {
                    "token set".to_string()
                } else {
                    "no token".to_string()
                },
            ];
            rows.push(self.render_row(
                ix + 1,
                server.name.clone(),
                meta,
                crate::icons::GLOBAL,
                Some(server.id.clone()),
                &theme,
                cx,
            ));
        }

        div()
            .id("servers-page")
            .size_full()
            .overflow_y_scroll()
            .child(
                widgets::page_column()
                    .child(widgets::page_header(
                        &theme,
                        "Servers",
                        (count > 0).then_some(count),
                    ))
                    .child(widgets::page_subtitle(
                        &theme,
                        "Engines on other machines this app drives directly over your \
                         network. Sessions, files and terminals then live on that machine.",
                    ))
                    .child(
                        div()
                            .mt(px(12.0))
                            .text_size(crate::typography::ui_rems(13.0))
                            .text_color(if status_is_error {
                                theme.danger
                            } else {
                                theme.text_muted
                            })
                            .child(status),
                    )
                    .child(widgets::section_card(&theme).children(rows))
                    .child(
                        div().mt(px(16.0)).flex().flex_row().child(
                            popover::btn_primary(&theme, "Add server")
                                .id("servers-add")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.open_add(window, cx);
                                })),
                        ),
                    ),
            )
            .when_some(dialog, |el, dialog| el.child(dialog))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_server;

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
