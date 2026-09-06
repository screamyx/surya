//! `Mail.Send`, `Mail.List`, `Mail.Ack` and the `WatchMail` subscription.
//! Same shape as [`crate::rpc::AuthRpc`]: a sub-service the engine's dispatcher
//! hands the mail methods to.

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use surya_rpc::{RpcError, RpcReply, RpcService, methods, parse_params};

use super::Mail;

#[derive(Clone)]
pub struct MailRpc {
    mail: Mail,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendParams {
    /// The sending agent's own chat. It must exist in the registry, and the
    /// envelope carries that chat's registry title rather than this string.
    #[serde(default)]
    from_chat: Option<String>,
    /// A sender the caller names outright — the CLI, a script.
    #[serde(default)]
    from: Option<String>,
    /// `agent-id`, a human alias, or `#workspace`.
    to: String,
    body: String,
    /// Reserved for cross-server mail. Anything but the local device stays
    /// queued until forwarding lands.
    #[serde(default)]
    to_device: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListParams {
    #[serde(default)]
    agent: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AckParams {
    id: String,
}

impl MailRpc {
    pub fn new(mail: Mail) -> Self {
        Self { mail }
    }

    pub fn handles(method: &str) -> bool {
        matches!(
            method,
            methods::MAIL_SEND | methods::MAIL_LIST | methods::MAIL_ACK | methods::WATCH_MAIL
        )
    }
}

#[async_trait]
impl RpcService for MailRpc {
    async fn handle(&self, method: &str, params: serde_json::Value) -> Result<RpcReply, RpcError> {
        let failed = |e: crate::EngineError| RpcError::Failed(e.to_string());
        match method {
            methods::MAIL_SEND => {
                let p: SendParams = parse_params(params)?;
                // `fromChat` is the agent's own session id, so the engine can
                // vouch for it. Anything else is a claim, and the envelope
                // says so.
                let receipt = match &p.from_chat {
                    Some(chat) => {
                        self.mail
                            .send_from_chat(chat, &p.to, &p.body, p.to_device.as_deref())
                            .await
                    }
                    None => {
                        self.mail
                            .send(
                                p.from.as_deref().unwrap_or("unknown"),
                                &p.to,
                                &p.body,
                                p.to_device.as_deref(),
                            )
                            .await
                    }
                }
                .map_err(failed)?;
                RpcReply::value(&receipt)
            }
            methods::MAIL_LIST => {
                let p: ListParams = parse_params(params).unwrap_or(ListParams { agent: None });
                let messages = self.mail.list(p.agent.as_deref()).await.map_err(failed)?;
                RpcReply::value(&serde_json::json!({ "messages": messages }))
            }
            methods::MAIL_ACK => {
                let p: AckParams = parse_params(params)?;
                let acked = self.mail.ack(&p.id).await.map_err(failed)?;
                RpcReply::value(&serde_json::json!({ "acked": acked }))
            }
            methods::WATCH_MAIL => {
                // Current value first, then one item per change — the same
                // contract as the other WATCH_* streams.
                let stream = futures::stream::unfold(
                    (self.mail.watch(), false),
                    |(mut rx, emitted)| async move {
                        if emitted {
                            rx.changed().await.ok()?;
                        }
                        let messages = rx.borrow_and_update().clone();
                        Some((serde_json::json!({ "messages": messages }), (rx, true)))
                    },
                );
                Ok(RpcReply::Stream(stream.boxed()))
            }
            _ => Err(RpcError::UnknownMethod(method.to_string())),
        }
    }
}
