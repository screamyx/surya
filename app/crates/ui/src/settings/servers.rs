//! Settings → Servers: the remote engines this app can drive directly over
//! the network (decision 18: LAN, VPN or tailnet, no cloud in between). Each
//! row is a `surya headless --bind <addr>` on another machine; its token comes
//! from `surya status` there. Connect swaps the engine the whole app talks
//! to. The shell owns the swap and persists the list, so this page only emits.

use gpui::{
    AnyElement, Context, Entity, EventEmitter, SharedString, Subscription, Window, div, prelude::*,
    px,
};
use crate::popover;
use crate::settings::ServerEntry;
use crate::state::{AppState, RemoteEngineTarget};
use crate::theme::Theme;

mod active;
mod add;
pub use active::{ActiveRow, active_row, canonical_url, status_text};
use add::AddDialog;
pub use add::parse_server;

/// The port a portless address means, in the Add dialog and when two
/// addresses are compared: one rule, so an engine saved from the Command
/// line row is found again by the address that was dialed. It is the port
/// deploy/install-engine.sh serves on; the loopback daemon's 27654 is never
/// what a remote entry means (#74).
pub const DEFAULT_PORT: u16 = 27700;

pub enum ServersEvent {
    /// The list or the active choice changed; the shell persists it.
    Changed {
        servers: Vec<ServerEntry>,
        active: Option<String>,
    },
    /// Dial this engine now (`None` = back to the local engine).
    Connect(Option<RemoteEngineTarget>),
}
/// What sits at the right edge of a Servers row.
enum RowTail {
    /// Not the engine in use: Connect dials the saved id (`None` = local).
    Connect(Option<String>),
    /// The engine in use.
    Active,
    /// The engine in use, known only from the command line: Active plus Save.
    ActiveUnsaved,
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
        self.add = Some(AddDialog::open(window, cx));
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

    /// The row that is the engine in use, from what the app was asked to dial.
    fn active_row(&self, cx: &Context<Self>) -> ActiveRow {
        let state = self.state.read(cx);
        let dialed = state.dialed_server().map(|target| target.url.as_str());
        active_row(dialed, self.active.as_deref(), &self.servers)
    }

    /// Keep the engine that came from the command line: a saved entry at its
    /// address, chosen, so the next launch (without the flag) dials it too.
    /// Idempotent: an entry already at that address is chosen, not added, and
    /// a target that would not save back to the same address is refused.
    fn save_command_line(&mut self, cx: &mut Context<Self>) {
        let Some(target) = self.state.read(cx).dialed_server().cloned() else {
            return;
        };
        let wanted = canonical_url(&target.url);
        if let Some(existing) = self.servers.iter().find(|s| canonical_url(&s.url()) == wanted) {
            self.active = Some(existing.id.clone());
            self.emit_changed(cx);
            cx.notify();
            return;
        }
        match add::entry_for_target(&target) {
            Ok(entry) => {
                self.active = Some(entry.id.clone());
                self.servers.push(entry);
                self.emit_changed(cx);
                cx.notify();
            }
            Err(message) => {
                tracing::warn!(url = %target.url, %message, "could not save the command-line engine");
            }
        }
    }

    /// `remove_id` is the saved entry the row's Remove button drops (`None`
    /// for rows that are not saved).
    fn render_row(
        &self,
        ix: usize,
        title: String,
        meta: Vec<String>,
        icon_path: &'static str,
        remove_id: Option<String>,
        tail: RowTail,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use crate::settings::widgets;
        let meta: Vec<AnyElement> = meta
            .into_iter()
            .map(|text| div().child(SharedString::from(text)).into_any_element())
            .collect();
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
        row = match tail {
            RowTail::Active => row.child(widgets::badge_active(theme, "Active")),
            RowTail::ActiveUnsaved => row.child(widgets::badge_active(theme, "Active")).child(
                widgets::ghost_action(theme)
                    .id(("server-save", ix))
                    .on_click(cx.listener(|this, _, _, cx| this.save_command_line(cx)))
                    .child(
                        crate::icons::icon(crate::icons::GLOBAL)
                            .size(px(14.0))
                            .text_color(theme.text_muted),
                    )
                    .child(SharedString::from("Save")),
            ),
            RowTail::Connect(connect_id) => row.child(
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
            ),
        };
        if let Some(remove_id) = remove_id {
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
        let active = self.active_row(cx);
        let (status, status_is_error) =
            status_text(&self.state.read(cx).connection, &active, &self.servers);
        let status = SharedString::from(status);
        let dialog = self
            .add
            .as_ref()
            .map(|dialog| add::render(dialog, window.viewport_size(), cx));
        let count = self.servers.len();
        let tail = |row_id: Option<String>, is_active: bool| {
            if is_active {
                RowTail::Active
            } else {
                RowTail::Connect(row_id)
            }
        };

        let mut rows: Vec<AnyElement> = vec![self.render_row(
            0,
            "This computer".into(),
            vec!["Engine started by this app, or a daemon on the local port".into()],
            crate::icons::MONITOR,
            None,
            tail(None, active == ActiveRow::Local),
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
            let is_active = active == ActiveRow::Saved(server.id.clone());
            rows.push(self.render_row(
                ix + 1,
                server.name.clone(),
                meta,
                crate::icons::GLOBAL,
                Some(server.id.clone()),
                tail(Some(server.id.clone()), is_active),
                &theme,
                cx,
            ));
        }
        // A `--engine` / `SURYA_ENGINE` target is dialed but never saved: give
        // it its own row so the badge does not fall on "This computer", and a
        // Save button so it can become a saved entry.
        if let ActiveRow::CommandLine(url) = &active {
            rows.push(self.render_row(
                rows.len(),
                "Command line".into(),
                vec![
                    url.clone(),
                    "from --engine or SURYA_ENGINE; Save keeps it".into(),
                ],
                crate::icons::GLOBAL,
                None,
                RowTail::ActiveUnsaved,
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
