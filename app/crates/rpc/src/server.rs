//! Server side: dispatch loop over string frames + the WebSocket acceptor.

use std::collections::HashMap;
use std::sync::Arc;

use futures::{SinkExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::handshake::server::{
    ErrorResponse, Request as HandshakeRequest, Response as HandshakeResponse,
};
use tokio_tungstenite::tungstenite::http::StatusCode;

use crate::{ClientFrame, RpcError, RpcReply, RpcService, ServerFrame};

/// Serve one connection: read client frames from `inbound`, write server frames to `out`.
/// Returns when `inbound` closes; all in-flight request tasks are aborted on exit.
pub async fn serve_connection(
    service: Arc<dyn RpcService>,
    out: mpsc::Sender<String>,
    mut inbound: mpsc::Receiver<String>,
) {
    let mut running: HashMap<u64, tokio::task::AbortHandle> = HashMap::new();
    while let Some(payload) = inbound.recv().await {
        // ndjson: a transport may batch several frames per message.
        for line in payload.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let frame: ClientFrame = match serde_json::from_str(line) {
                Ok(frame) => frame,
                Err(err) => {
                    tracing::warn!(error = %err, "rpc: dropping malformed client frame");
                    continue;
                }
            };
            running.retain(|_, task| !task.is_finished());
            if frame.cancel {
                if let Some(task) = running.remove(&frame.id) {
                    task.abort();
                }
                continue;
            }
            let Some(method) = frame.method else {
                tracing::warn!(id = frame.id, "rpc: frame has neither method nor cancel");
                continue;
            };
            let task = tokio::spawn(handle_request(
                service.clone(),
                out.clone(),
                frame.id,
                method,
                frame.params,
            ));
            running.insert(frame.id, task.abort_handle());
        }
    }
    for (_, task) in running {
        task.abort();
    }
}

async fn handle_request(
    service: Arc<dyn RpcService>,
    out: mpsc::Sender<String>,
    id: u64,
    method: String,
    params: serde_json::Value,
) {
    let send = |frame: ServerFrame| {
        let out = out.clone();
        async move {
            match serde_json::to_string(&frame) {
                Ok(json) => out.send(json).await.map_err(|_| RpcError::Closed),
                Err(err) => {
                    tracing::error!(error = %err, "rpc: failed to serialize server frame");
                    Err(RpcError::Closed)
                }
            }
        }
    };
    match service.handle(&method, params).await {
        Ok(RpcReply::Value(value)) => {
            let _ = send(ServerFrame {
                id,
                ok: Some(value),
                ..Default::default()
            })
            .await;
        }
        Ok(RpcReply::Stream(mut stream)) => {
            // Only the versioned checkout-PR stream uses an explicit readiness
            // frame. Sending it for legacy streams would make older clients remove
            // their pending stream as if it were a unary response.
            if method == crate::methods::WATCH_CHECKOUT_CHANGE_REQUEST
                && send(ServerFrame {
                    id,
                    ok: Some(serde_json::json!({ "stream": true })),
                    ..Default::default()
                })
                .await
                .is_err()
            {
                return;
            }
            while let Some(item) = stream.next().await {
                if send(ServerFrame {
                    id,
                    item: Some(item),
                    ..Default::default()
                })
                .await
                .is_err()
                {
                    return; // connection gone
                }
            }
            let _ = send(ServerFrame {
                id,
                done: true,
                ..Default::default()
            })
            .await;
        }
        Err(err) => {
            let _ = send(ServerFrame {
                id,
                err: Some(err.to_string()),
                ..Default::default()
            })
            .await;
        }
    }
}

/// Accept WebSocket connections forever, serving each with `service`.
/// Open socket: no token check. Only ever bind this to loopback.
pub async fn serve_ws_listener(listener: TcpListener, service: Arc<dyn RpcService>) {
    serve_ws_listener_with_auth(listener, service, None).await
}

/// Like [`serve_ws_listener`], but when `token` is `Some` every handshake must
/// carry `Authorization: Bearer <token>` or it is refused with 401. This is
/// what makes a non-loopback bind safe: the socket is reachable over the
/// network, so possession of the shared token is the whole gate.
pub async fn serve_ws_listener_with_auth(
    listener: TcpListener,
    service: Arc<dyn RpcService>,
    token: Option<Arc<str>>,
) {
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                tracing::debug!(%peer, "rpc: tcp connection accepted");
                tokio::spawn(serve_ws_socket(stream, peer, service.clone(), token.clone()));
            }
            Err(err) => {
                tracing::warn!(error = %err, "rpc: accept failed");
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }
}

/// Constant-time check of an `Authorization` header value against the shared
/// token. Accepts `Bearer <token>` only; a missing header never matches.
pub fn authorization_matches(header: Option<&[u8]>, token: &str) -> bool {
    let Some(header) = header else {
        return false;
    };
    let Some(presented) = header
        .strip_prefix(b"Bearer ")
        .or_else(|| header.strip_prefix(b"bearer "))
    else {
        return false;
    };
    let presented = presented.trim_ascii();
    let expected = token.as_bytes();
    // Length leaks nothing useful (the token length is fixed by the generator),
    // but the byte comparison itself must not short-circuit.
    if presented.len() != expected.len() {
        return false;
    }
    presented
        .iter()
        .zip(expected)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

async fn serve_ws_socket(
    stream: TcpStream,
    peer: std::net::SocketAddr,
    service: Arc<dyn RpcService>,
    token: Option<Arc<str>>,
) {
    // Native viewports dial this socket with a bare `connect_async` and send
    // no `Origin` header. A browser always attaches `Origin` to a WebSocket
    // handshake and cannot forge or suppress it from script, and WebSockets
    // are exempt from the Same-Origin Policy — so only rejecting any handshake
    // that carries `Origin` keeps a page the user happens to visit from
    // reaching this local socket. Keep this check.
    //
    // The large `Err` (ErrorResponse) is the shape tungstenite's Callback
    // trait requires; it can't be boxed away here.
    let auth_label = if token.is_some() { "token" } else { "open" };
    #[allow(clippy::result_large_err)]
    let gate = |req: &HandshakeRequest, resp: HandshakeResponse| {
        if let Some(origin) = req.headers().get("origin") {
            tracing::warn!(
                %peer,
                origin = %String::from_utf8_lossy(origin.as_bytes()),
                "rpc: rejecting handshake carrying an Origin header (cross-origin browser dial)"
            );
            let mut err = ErrorResponse::new(Some("origin not allowed on local IPC".to_string()));
            *err.status_mut() = StatusCode::FORBIDDEN;
            return Err(err);
        }
        if let Some(token) = token.as_deref() {
            let header = req.headers().get("authorization").map(|v| v.as_bytes());
            if !authorization_matches(header, token) {
                tracing::warn!(%peer, "rpc: rejecting handshake without a valid IPC token");
                let mut err = ErrorResponse::new(Some("ipc token required".to_string()));
                *err.status_mut() = StatusCode::UNAUTHORIZED;
                return Err(err);
            }
        }
        Ok(resp)
    };
    let ws = match tokio_tungstenite::accept_hdr_async(stream, gate).await {
        Ok(ws) => ws,
        Err(err) => {
            tracing::warn!(%peer, error = %err, "rpc: websocket handshake failed");
            return;
        }
    };
    // The one line to grep for when proving a remote viewport reached this
    // engine: peer address plus which gate admitted it.
    tracing::info!(%peer, auth = auth_label, "rpc: connection accepted");
    let (mut sink, mut ws_stream) = ws.split();
    let (out_tx, mut out_rx) = mpsc::channel::<String>(256);
    let (in_tx, in_rx) = mpsc::channel::<String>(256);

    // Pump: socket <-> string channels. Ends when either side closes.
    let pump = tokio::spawn(async move {
        loop {
            tokio::select! {
                frame = out_rx.recv() => match frame {
                    Some(text) => {
                        if sink.send(WsMessage::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    None => {
                        let _ = sink.send(WsMessage::Close(None)).await;
                        break;
                    }
                },
                message = ws_stream.next() => match message {
                    Some(Ok(WsMessage::Text(text))) => {
                        if in_tx.send(text).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) | Some(Err(_)) | None => break,
                    Some(Ok(_)) => {} // ping/pong/binary — ignored
                },
            }
        }
    });

    serve_connection(service, out_tx, in_rx).await;
    pump.abort();
    tracing::info!(%peer, "rpc: connection closed");
}

#[cfg(test)]
mod auth_tests {
    use super::authorization_matches;

    #[test]
    fn bearer_token_must_match_exactly() {
        assert!(authorization_matches(Some(b"Bearer abc123"), "abc123"));
        assert!(authorization_matches(Some(b"bearer abc123"), "abc123"));
        assert!(!authorization_matches(Some(b"Bearer abc124"), "abc123"));
        assert!(!authorization_matches(Some(b"Bearer abc12"), "abc123"));
        assert!(!authorization_matches(Some(b"abc123"), "abc123"));
        assert!(!authorization_matches(Some(b"Basic abc123"), "abc123"));
        assert!(!authorization_matches(None, "abc123"));
    }
}
