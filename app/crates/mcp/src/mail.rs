//! `send_message`: agent-to-agent mail (decision 19, thin).
//!
//! The daemon owns delivery. This server only hands the message over: it
//! writes one JSON line to the daemon's Unix socket, and falls back to an
//! append-only log the daemon drains when the socket is not there. Either way
//! the agent gets a `delivery_id` back immediately - the tool never blocks on
//! the recipient.

use serde_json::{Value, json};

use crate::config::{Config, append_line};

/// Where the message went. Reported back to the agent so a fallback is
/// visible rather than silent.
#[derive(Debug, PartialEq)]
pub enum Route {
    Socket,
    Log,
}

impl Route {
    pub fn as_str(&self) -> &'static str {
        match self {
            Route::Socket => "socket",
            Route::Log => "log",
        }
    }
}

#[derive(Debug)]
pub struct Delivery {
    pub delivery_id: String,
    pub route: Route,
}

/// An address is an agent id, `#workspace` or `#server` (decision 19).
fn validate_address(to: &str) -> Result<(), String> {
    if to.trim().is_empty() {
        return Err("send_message needs a \"to\" address: an agent id, #workspace or #server".into());
    }
    if to.contains(char::is_whitespace) {
        return Err(format!("\"{to}\" is not an address; addresses carry no spaces"));
    }
    Ok(())
}

#[cfg(unix)]
fn write_to_socket(path: &std::path::Path, line: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::net::UnixStream;
    let mut stream = UnixStream::connect(path)?;
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()
}

#[cfg(not(unix))]
fn write_to_socket(_path: &std::path::Path, _line: &str) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "unix sockets are not available on this platform",
    ))
}

/// Handle one `send_message` call.
pub fn send(config: &Config, arguments: &Value) -> Result<Delivery, String> {
    let to = arguments.get("to").and_then(Value::as_str).unwrap_or("");
    validate_address(to)?;
    let text = arguments.get("text").and_then(Value::as_str).unwrap_or("");
    if text.trim().is_empty() {
        return Err("send_message needs a non-empty \"text\"".into());
    }
    let delivery_id = format!("d_{}", uuid::Uuid::new_v4().simple());
    let record = json!({
        "delivery_id": delivery_id,
        "from": config.agent_id,
        "to": to,
        "workspace": config.workspace,
        "text": text,
        "at": chrono::Utc::now().to_rfc3339(),
    });
    let line = record.to_string();
    if write_to_socket(&config.mail_socket, &line).is_ok() {
        return Ok(Delivery {
            delivery_id,
            route: Route::Socket,
        });
    }
    append_line(&config.mail_log, &line)
        .map_err(|e| format!("could not write mail to {:?}: {e}", config.mail_log))?;
    Ok(Delivery {
        delivery_id,
        route: Route::Log,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn config(dir: &std::path::Path) -> Config {
        Config {
            agent_id: "seat-1".into(),
            workspace: "demo".into(),
            workspace_root: dir.to_path_buf(),
            card_store: dir.join("cards.jsonl"),
            mail_socket: dir.join("absent.sock"),
            mail_log: dir.join("mail.jsonl"),
        }
    }

    #[test]
    fn one_send_writes_one_record_to_the_fallback_log() {
        let dir = tempfile::tempdir().unwrap();
        let config = config(dir.path());
        let asked = 1;
        let delivery = send(
            &config,
            &json!({"to": "#demo", "text": "the migration is ready"}),
        )
        .unwrap();
        assert_eq!(delivery.route, Route::Log);
        let body = std::fs::read_to_string(&config.mail_log).unwrap();
        let written = body.lines().count();
        assert_eq!((asked, written), (1, 1), "asked={asked} written={written}");
        let record: Value = serde_json::from_str(body.lines().next().unwrap()).unwrap();
        assert_eq!(record["to"], "#demo");
        assert_eq!(record["from"], "seat-1");
        assert_eq!(record["delivery_id"], delivery.delivery_id);
    }

    #[cfg(unix)]
    #[test]
    fn the_socket_wins_when_the_daemon_is_listening() {
        use std::io::{BufRead, BufReader};
        use std::os::unix::net::UnixListener;

        let dir = tempfile::tempdir().unwrap();
        let mut config = config(dir.path());
        config.mail_socket = dir.path().join("mail.sock");
        let listener = UnixListener::bind(&config.mail_socket).unwrap();
        let reader = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).unwrap();
            line
        });

        let delivery = send(&config, &json!({"to": "seat-2", "text": "hello"})).unwrap();
        assert_eq!(delivery.route, Route::Socket);
        let line = reader.join().unwrap();
        let record: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(record["to"], "seat-2");
        assert_eq!(record["delivery_id"], delivery.delivery_id);
        assert!(
            !config.mail_log.exists(),
            "the fallback log stays untouched when the socket takes it"
        );
    }

    #[test]
    fn an_empty_address_or_body_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let config = config(dir.path());
        assert!(send(&config, &json!({"to": "", "text": "x"})).is_err());
        assert!(send(&config, &json!({"to": "seat-2", "text": "  "})).is_err());
        assert!(send(&config, &json!({"to": "two words", "text": "x"})).is_err());
        assert!(
            !PathBuf::from(&config.mail_log).exists(),
            "a refused send writes nothing"
        );
    }
}
