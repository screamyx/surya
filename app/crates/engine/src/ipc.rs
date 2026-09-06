//! IPC bind + shared-token policy for the engine's WebSocket RPC socket.
//!
//! Loopback stays exactly as it always was: `127.0.0.1:<port>`, no token.
//! Binding anywhere else (a LAN, VPN or tailnet address, or `0.0.0.0`) puts
//! the socket on the network, so it requires a shared token. The token is
//! `SURYA_IPC_TOKEN` when set, otherwise `{data_dir}/ipc-token`, generated on
//! the first non-loopback start and printed by `surya status`. A remote
//! viewport presents it as `Authorization: Bearer <token>`.
//!
//! No TLS here on purpose: the networks this is meant for (tailnet, VPN, LAN)
//! carry their own encryption, and a relay-free direct dial is decision 18.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// File under the data dir holding the generated token (mode 0600 on unix).
pub const TOKEN_FILE: &str = "ipc-token";

/// Environment variable naming the bind address (`SURYA_BIND`).
pub const BIND_ENV: &str = "SURYA_BIND";

/// Environment variable carrying an explicit token (`SURYA_IPC_TOKEN`).
pub const TOKEN_ENV: &str = "SURYA_IPC_TOKEN";

pub const DEFAULT_BIND: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Where and how the engine serves its RPC socket.
#[derive(Debug, Clone)]
pub struct IpcConfig {
    pub bind: IpAddr,
    pub port: u16,
    /// Explicit token (env). `None` falls back to the data-dir file when the
    /// bind is not loopback.
    pub token: Option<String>,
    pub data_dir: PathBuf,
}

impl IpcConfig {
    pub fn is_loopback(&self) -> bool {
        self.bind.is_loopback()
    }

    /// Whether handshakes on this socket must carry the token.
    pub fn enforces_token(&self) -> bool {
        self.token.is_some() || !self.is_loopback()
    }

    /// The token this engine enforces, generating the data-dir file when a
    /// non-loopback bind has none yet. `Ok(None)` means an open loopback socket.
    pub fn resolve_token(&self) -> std::io::Result<Option<String>> {
        if let Some(token) = self.token.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            return Ok(Some(token.to_string()));
        }
        if self.is_loopback() {
            return Ok(None);
        }
        load_or_create_token(&self.data_dir).map(Some)
    }

    /// The address a local CLI (`surya status`, `surya sync`) dials to reach
    /// this engine: the bind address itself, or loopback for a wildcard bind.
    pub fn dial_addr(&self) -> SocketAddr {
        let host = if self.bind.is_unspecified() {
            match self.bind {
                IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
                IpAddr::V6(_) => IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
            }
        } else {
            self.bind
        };
        SocketAddr::new(host, self.port)
    }

    pub fn dial_url(&self) -> String {
        format!("ws://{}", self.dial_addr())
    }

    /// Dial this engine from the same machine, presenting the token when one
    /// is enforced.
    pub async fn connect(&self) -> Result<surya_rpc::RpcClient, surya_rpc::RpcError> {
        let token = self
            .resolve_token()
            .map_err(|e| surya_rpc::RpcError::Transport(format!("ipc token: {e}")))?;
        surya_rpc::connect_ws_with_token(&self.dial_url(), token.as_deref()).await
    }
}

/// Parse `SURYA_BIND`-style input. Empty or unset means loopback.
pub fn parse_bind(value: Option<&str>) -> Result<IpAddr, String> {
    match value.map(str::trim).filter(|v| !v.is_empty()) {
        None => Ok(DEFAULT_BIND),
        Some(raw) => raw
            .parse::<IpAddr>()
            .map_err(|_| format!("{BIND_ENV}={raw:?} is not an IP address (use 0.0.0.0 for every interface)")),
    }
}

pub fn token_path(data_dir: &Path) -> PathBuf {
    data_dir.join(TOKEN_FILE)
}

/// 256 bits from the OS RNG as 64 hex chars (two v4 uuids, hyphens dropped).
pub fn generate_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

/// Read `{data_dir}/ipc-token`, creating it (0600) on first use.
pub fn load_or_create_token(data_dir: &Path) -> std::io::Result<String> {
    let path = token_path(data_dir);
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let existing = existing.trim();
        if !existing.is_empty() {
            return Ok(existing.to_string());
        }
    }
    std::fs::create_dir_all(data_dir)?;
    let token = generate_token();
    write_private(&path, &token)?;
    tracing::info!(path = %path.display(), "IPC token generated; `surya status` prints it");
    Ok(token)
}

fn write_private(path: &Path, contents: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        use std::io::Write;
        let mut file = options.open(&tmp)?;
        file.write_all(contents.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    }
    std::fs::rename(&tmp, path)
}

/// Bind the RPC socket and serve it forever on a tokio task.
///
/// A non-loopback bind without a token refuses to start: an open engine on a
/// network address would let anyone on it run agents as this user.
pub async fn serve(
    config: &IpcConfig,
    service: Arc<dyn surya_rpc::RpcService>,
) -> std::io::Result<tokio::task::JoinHandle<()>> {
    let token = config.resolve_token()?;
    if !config.is_loopback() && token.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "refusing to bind {} without an IPC token; set {TOKEN_ENV} or keep the bind on loopback",
                config.bind
            ),
        ));
    }
    let listener = tokio::net::TcpListener::bind((config.bind, config.port)).await?;
    let addr = listener.local_addr()?;
    tracing::info!(
        %addr,
        auth = if token.is_some() { "token" } else { "open" },
        "IPC server listening"
    );
    let token: Option<Arc<str>> = token.map(Arc::from);
    Ok(tokio::spawn(surya_rpc::serve_ws_listener_with_auth(
        listener, service, token,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_parses_and_defaults_to_loopback() {
        assert_eq!(parse_bind(None).unwrap(), DEFAULT_BIND);
        assert_eq!(parse_bind(Some("  ")).unwrap(), DEFAULT_BIND);
        assert_eq!(
            parse_bind(Some("100.64.0.9")).unwrap(),
            "100.64.0.9".parse::<IpAddr>().unwrap()
        );
        assert!(parse_bind(Some("tailnet-host")).is_err());
    }

    #[test]
    fn loopback_without_token_stays_open() {
        let dir = tempfile::tempdir().unwrap();
        let config = IpcConfig {
            bind: DEFAULT_BIND,
            port: 0,
            token: None,
            data_dir: dir.path().to_path_buf(),
        };
        assert!(!config.enforces_token());
        assert_eq!(config.resolve_token().unwrap(), None);
        assert!(!token_path(dir.path()).exists());
    }

    #[test]
    fn network_bind_generates_a_token_once() {
        let dir = tempfile::tempdir().unwrap();
        let config = IpcConfig {
            bind: "0.0.0.0".parse().unwrap(),
            port: 0,
            token: None,
            data_dir: dir.path().to_path_buf(),
        };
        assert!(config.enforces_token());
        let first = config.resolve_token().unwrap().unwrap();
        let second = config.resolve_token().unwrap().unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
        assert_eq!(config.dial_addr().ip(), DEFAULT_BIND);
    }

    #[test]
    fn explicit_token_wins_over_the_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(token_path(dir.path()), "from-file\n").unwrap();
        let config = IpcConfig {
            bind: DEFAULT_BIND,
            port: 0,
            token: Some("from-env".into()),
            data_dir: dir.path().to_path_buf(),
        };
        assert!(config.enforces_token());
        assert_eq!(config.resolve_token().unwrap().as_deref(), Some("from-env"));
    }

    #[tokio::test]
    async fn network_bind_refuses_when_no_token_can_be_had() {
        // A data dir that cannot exist (its parent is a file) means no token
        // file can be generated, so a wildcard bind must not come up.
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, "x").unwrap();
        let config = IpcConfig {
            bind: "0.0.0.0".parse().unwrap(),
            port: 0,
            token: None,
            data_dir: blocker.join("child"),
        };
        assert!(serve(&config, Arc::new(Denied)).await.is_err());

        // The same bind with an explicit token comes up.
        let config = IpcConfig {
            token: Some("t".into()),
            ..config
        };
        let task = serve(&config, Arc::new(Denied)).await.unwrap();
        task.abort();
    }

    #[tokio::test]
    async fn token_gate_admits_only_the_bearer() {
        let dir = tempfile::tempdir().unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let config = IpcConfig {
            bind: DEFAULT_BIND,
            port,
            token: Some("secret-token".into()),
            data_dir: dir.path().to_path_buf(),
        };
        let task = serve(&config, Arc::new(Denied)).await.unwrap();
        let url = format!("ws://127.0.0.1:{port}");
        assert!(surya_rpc::connect_ws(&url).await.is_err(), "no token must be refused");
        assert!(
            surya_rpc::connect_ws_with_token(&url, Some("wrong")).await.is_err(),
            "wrong token must be refused"
        );
        let client = surya_rpc::connect_ws_with_token(&url, Some("secret-token"))
            .await
            .expect("right token connects");
        let err = client.call("Anything", serde_json::json!({})).await.unwrap_err();
        assert!(matches!(err, surya_rpc::RpcError::UnknownMethod(_)));
        task.abort();
    }

    struct Denied;

    #[async_trait::async_trait]
    impl surya_rpc::RpcService for Denied {
        async fn handle(
            &self,
            method: &str,
            _params: serde_json::Value,
        ) -> Result<surya_rpc::RpcReply, surya_rpc::RpcError> {
            Err(surya_rpc::RpcError::UnknownMethod(method.to_string()))
        }
    }
}
