//! Which Servers row is the engine in use, and the one-line status above the
//! list. Pure functions so the rules are testable without a window.
//!
//! The decision starts from what the app was asked to dial (`--engine`,
//! `ZERON_ENGINE`, or the saved active server), not from the engine's
//! transport: the loopback daemon is also reached over a websocket, and
//! reading the transport put the badge on a bogus "Command line" row for it
//! (review of PR #68, 2026-09-05).

use crate::settings::ServerEntry;
use crate::state::ConnectionStatus;

/// The row that carries the Active badge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveRow {
    /// Nothing was dialed: the embedded engine or the loopback daemon.
    Local,
    /// The saved entry the dialed address belongs to.
    Saved(String),
    /// A dialed address no saved entry knows: the command line or the
    /// environment (on Windows also the launcher's servers.json).
    CommandLine(String),
}

/// `ws://Host:27700/` and `WS://host:27700` name the same engine: lower-case
/// the scheme and the host, make the scheme's default port explicit, drop a
/// bare trailing slash. A path or query is kept as typed, and `wss://` stays
/// distinct from `ws://` because it is a different endpoint.
pub fn canonical_url(url: &str) -> String {
    let url = url.trim();
    let (scheme, rest) = match url.split_once("://") {
        Some((scheme, rest)) => (scheme.to_ascii_lowercase(), rest),
        None => ("ws".to_string(), url),
    };
    let (authority, path) = match rest.find('/') {
        Some(ix) => (&rest[..ix], &rest[ix..]),
        None => (rest, ""),
    };
    let path = if path == "/" { "" } else { path };
    let (host, port) = split_host_port(authority);
    let port = port.unwrap_or(if scheme == "wss" { "443" } else { "80" });
    format!("{scheme}://{}:{port}{path}", host.to_ascii_lowercase())
}

/// `host:port`, `[v6]:port`, `host`, `[v6]`, or a bare IPv6 address.
fn split_host_port(authority: &str) -> (&str, Option<&str>) {
    if let Some(end) = authority
        .strip_prefix('[')
        .and_then(|_| authority.find(']'))
    {
        let host = &authority[..=end];
        let port = authority[end + 1..]
            .strip_prefix(':')
            .filter(|p| !p.is_empty());
        return (host, port);
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') && !port.is_empty() => (host, Some(port)),
        _ => (authority, None),
    }
}

/// Which row is active. `dialed` is the address the app was asked to reach
/// (`None` for the local engine, daemon or embedded); `active_id` is the
/// saved entry the user picked, which wins over an address match so two
/// entries at the same address badge the one that was chosen.
pub fn active_row(
    dialed: Option<&str>,
    active_id: Option<&str>,
    servers: &[ServerEntry],
) -> ActiveRow {
    let Some(dialed) = dialed else {
        return ActiveRow::Local;
    };
    let wanted = canonical_url(dialed);
    let same_address = |entry: &&ServerEntry| canonical_url(&entry.url()) == wanted;
    let chosen = active_id
        .and_then(|id| servers.iter().find(|entry| entry.id == id))
        .filter(same_address);
    match chosen.or_else(|| servers.iter().find(same_address)) {
        Some(entry) => ActiveRow::Saved(entry.id.clone()),
        None => ActiveRow::CommandLine(dialed.to_string()),
    }
}

/// One line of truth about what the app is talking to right now, and whether
/// it is an error.
pub fn status_text(
    connection: &ConnectionStatus,
    active: &ActiveRow,
    servers: &[ServerEntry],
) -> (String, bool) {
    match connection {
        ConnectionStatus::Connecting => ("Connecting\u{2026}".into(), false),
        ConnectionStatus::Failed(message) => (format!("Not connected: {message}"), true),
        ConnectionStatus::Ready => match active {
            ActiveRow::Local => ("Using the engine on this computer".into(), false),
            ActiveRow::Saved(id) => match servers.iter().find(|entry| &entry.id == id) {
                Some(entry) => (
                    format!("Connected to {} ({})", entry.name, entry.url()),
                    false,
                ),
                None => ("Connected to a saved server".into(), false),
            },
            ActiveRow::CommandLine(url) => (format!("Connected to {url} (from --engine)"), false),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, host: &str, port: u16) -> ServerEntry {
        ServerEntry {
            id: id.into(),
            name: format!("name-{id}"),
            host: host.into(),
            port,
            token: None,
        }
    }

    #[test]
    fn canonical_url_folds_case_default_port_and_trailing_slash() {
        assert_eq!(canonical_url("ws://PC-Ajim:27700/"), "ws://pc-ajim:27700");
        assert_eq!(canonical_url("WS://pc-ajim:27700"), "ws://pc-ajim:27700");
        assert_eq!(canonical_url("ws://pc-ajim"), "ws://pc-ajim:80");
        assert_eq!(canonical_url("wss://pc-ajim"), "wss://pc-ajim:443");
        assert_eq!(canonical_url("pc-ajim:27700"), "ws://pc-ajim:27700");
        assert_eq!(canonical_url("ws://[::1]:27700"), "ws://[::1]:27700");
        assert_eq!(canonical_url("ws://[::1]"), "ws://[::1]:80");
        assert_eq!(canonical_url("ws://h:1/rpc?x=1"), "ws://h:1/rpc?x=1");
    }

    /// The loopback daemon is reached over a websocket too; only a dialed
    /// address makes a row remote (asked=3 local=3).
    #[test]
    fn nothing_dialed_is_local_whatever_the_transport_or_saved_id() {
        let servers = vec![entry("s1", "127.0.0.1", 27654)];
        assert_eq!(active_row(None, None, &servers), ActiveRow::Local);
        assert_eq!(active_row(None, Some("s1"), &servers), ActiveRow::Local);
        assert_eq!(active_row(None, Some("gone"), &[]), ActiveRow::Local);
    }

    #[test]
    fn the_chosen_entry_wins_over_an_address_match() {
        let servers = vec![
            entry("first", "pc-ajim", 27700),
            entry("second", "pc-ajim", 27700),
        ];
        assert_eq!(
            active_row(Some("ws://pc-ajim:27700"), Some("second"), &servers),
            ActiveRow::Saved("second".into())
        );
        assert_eq!(
            active_row(Some("ws://pc-ajim:27700"), None, &servers),
            ActiveRow::Saved("first".into())
        );
        // A chosen entry at another address is stale: fall back to the address.
        let servers = vec![
            entry("first", "pc-ajim", 27700),
            entry("other", "build-box", 1),
        ];
        assert_eq!(
            active_row(Some("ws://pc-ajim:27700"), Some("other"), &servers),
            ActiveRow::Saved("first".into())
        );
    }

    #[test]
    fn addresses_compare_after_normalising() {
        let servers = vec![entry("s1", "PC-Ajim", 27700), entry("s2", "build-box", 80)];
        for dialed in [
            "ws://pc-ajim:27700",
            "ws://pc-ajim:27700/",
            "WS://PC-AJIM:27700",
        ] {
            assert_eq!(
                active_row(Some(dialed), None, &servers),
                ActiveRow::Saved("s1".into()),
                "{dialed}"
            );
        }
        assert_eq!(
            active_row(Some("ws://build-box"), None, &servers),
            ActiveRow::Saved("s2".into())
        );
        assert_eq!(
            active_row(Some("wss://pc-ajim:27700"), None, &servers),
            ActiveRow::CommandLine("wss://pc-ajim:27700".into())
        );
    }

    #[test]
    fn an_unknown_address_is_the_command_line() {
        let servers = vec![entry("s1", "pc-ajim", 27700)];
        assert_eq!(
            active_row(Some("ws://100.83.77.3:27700"), Some("s1"), &servers),
            ActiveRow::CommandLine("ws://100.83.77.3:27700".into())
        );
        assert_eq!(
            active_row(Some("ws://x:1"), None, &[]),
            ActiveRow::CommandLine("ws://x:1".into())
        );
    }

    #[test]
    fn status_text_names_the_engine_in_use() {
        let servers = vec![entry("s1", "pc-ajim", 27700)];
        let ready = ConnectionStatus::Ready;
        assert_eq!(
            status_text(&ConnectionStatus::Connecting, &ActiveRow::Local, &servers),
            ("Connecting\u{2026}".to_string(), false)
        );
        assert_eq!(
            status_text(
                &ConnectionStatus::Failed("boom".into()),
                &ActiveRow::Saved("s1".into()),
                &servers
            ),
            ("Not connected: boom".to_string(), true)
        );
        assert_eq!(
            status_text(&ready, &ActiveRow::Local, &servers),
            ("Using the engine on this computer".to_string(), false)
        );
        assert_eq!(
            status_text(&ready, &ActiveRow::Saved("s1".into()), &servers),
            (
                "Connected to name-s1 (ws://pc-ajim:27700)".to_string(),
                false
            )
        );
        assert_eq!(
            status_text(&ready, &ActiveRow::CommandLine("ws://x:1".into()), &servers),
            ("Connected to ws://x:1 (from --engine)".to_string(), false)
        );
    }
}
