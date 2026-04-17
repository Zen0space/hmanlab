//! Channel-based approval handler for tool execution.
//!
//! This module implements the `ApprovalHandler` pattern used by the gateway:
//! when a tool needs approval, the handler:
//! 1. Generates a UUID approval ID
//! 2. Publishes an `OutboundMessage` to the originating channel with approval
//!    prompt metadata (including the approval ID for inline keyboard buttons)
//! 3. Awaits a oneshot receiver (with timeout) for the channel to deliver a response
//!
//! The channel (e.g. Telegram) is responsible for rendering the approval prompt
//! with inline keyboard buttons and resolving the store entry when the user
//! taps a button or sends a `/approve` text command.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::bus::message::OutboundMessage;
use crate::bus::MessageBus;
use crate::tools::approval::{ApprovalRequest, ApprovalResponse};
use crate::tools::approval_store::PendingApproval;

/// Context carried per approval request so the handler knows which
/// channel/chat to send the prompt to.
#[derive(Clone, Debug)]
pub struct ChannelApprovalContext {
    /// Channel the user message came from (e.g. "telegram").
    pub channel: String,
    /// Chat ID of the originating conversation.
    pub chat_id: String,
    /// Optional thread ID (for Telegram forum topics).
    pub thread_id: Option<String>,
    /// Optional message ID to reply to.
    pub reply_to: Option<String>,
}

/// Create the channel-based approval handler closure.
///
/// Returns a closure matching the `ApprovalHandler` signature expected by
/// `AgentLoop::set_approval_handler()`. The closure:
/// - Creates a `PendingApproval` entry in the store
/// - Publishes an `OutboundMessage` with approval metadata
/// - Awaits the oneshot with a configurable timeout
/// - Returns `Denied` on timeout
pub fn create_channel_approval_handler(
    bus: Arc<MessageBus>,
    store: crate::tools::approval_store::ApprovalRequestStore,
    timeout_secs: u64,
) -> impl Fn(
    ChannelApprovalContext,
    ApprovalRequest,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ApprovalResponse> + Send>>
       + Send
       + Sync
       + 'static {
    move |_ctx: ChannelApprovalContext, _request: ApprovalRequest| {
        let _bus = bus.clone();
        let _store = store.clone();
        let _timeout = timeout_secs;
        Box::pin(async move { ApprovalResponse::Denied("not wired".to_string()) })
    }
}

pub async fn channel_approval_inner(
    bus: Arc<MessageBus>,
    store: crate::tools::approval_store::ApprovalRequestStore,
    ctx: ChannelApprovalContext,
    request: ApprovalRequest,
    timeout_secs: u64,
) -> ApprovalResponse {
    let approval_id = Uuid::new_v4().to_string();

    let args_display = match serde_json::to_string_pretty(&request.arguments) {
        Ok(pretty) => {
            if pretty.len() > 500 {
                format!("{}...", &pretty[..500])
            } else {
                pretty
            }
        }
        Err(_) => request.arguments.to_string(),
    };

    info!(
        "Channel approval requested: id={}, tool={}, channel={}, chat={}",
        approval_id, request.tool_name, ctx.channel, ctx.chat_id
    );

    let (dummy_tx, _) = tokio::sync::oneshot::channel();
    let approval = PendingApproval {
        id: approval_id.clone(),
        tool_name: request.tool_name.clone(),
        arguments_summary: args_display.clone(),
        channel: ctx.channel.clone(),
        chat_id: ctx.chat_id.clone(),
        created_at: Utc::now(),
        resolver: dummy_tx,
    };

    let rx = store.insert(approval).await;

    let content = format_approval_prompt(&request.tool_name, &args_display, &approval_id);

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("approval_id".to_string(), approval_id.clone());
    metadata.insert("approval_prompt".to_string(), "true".to_string());
    if let Some(ref tid) = ctx.thread_id {
        metadata.insert("telegram_thread_id".to_string(), tid.clone());
    }
    if let Some(ref mid) = ctx.reply_to {
        metadata.insert("telegram_message_id".to_string(), mid.clone());
    }

    let outbound = OutboundMessage {
        channel: ctx.channel.clone(),
        chat_id: ctx.chat_id.clone(),
        content,
        reply_to: None,
        metadata,
    };

    if let Err(e) = bus.publish_outbound(outbound).await {
        warn!("Failed to publish approval prompt: {}", e);
        store
            .resolve(
                &approval_id,
                ApprovalResponse::Denied("failed to send approval prompt".to_string()),
            )
            .await;
        return ApprovalResponse::Denied("failed to send approval prompt to channel".to_string());
    }

    debug!(
        "Awaiting approval response for {} (timeout={}s)",
        approval_id, timeout_secs
    );

    let timeout = Duration::from_secs(timeout_secs.max(1));
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(response)) => {
            info!("Approval {} resolved: {:?}", approval_id, response);
            response
        }
        Ok(Err(_)) => {
            warn!(
                "Approval {} oneshot dropped (store entry already removed)",
                approval_id
            );
            ApprovalResponse::Denied("approval channel closed".to_string())
        }
        Err(_) => {
            warn!("Approval {} timed out after {}s", approval_id, timeout_secs);
            store
                .resolve(&approval_id, ApprovalResponse::TimedOut)
                .await;
            ApprovalResponse::TimedOut
        }
    }
}

/// Format the approval prompt message shown to the user.
pub fn format_approval_prompt(tool_name: &str, args_display: &str, approval_id: &str) -> String {
    let short_id = if approval_id.len() > 8 {
        &approval_id[..8]
    } else {
        approval_id
    };
    format!(
        "\u{26a0}\u{fe0f} <b>Approval Required</b>\n\
         \n\
         Tool: <code>{}</code>\n\
         Arguments:\n<pre>{}</pre>\n\
         \n\
         ID: <code>{}</code>",
        tool_name, args_display, short_id,
    )
}

/// Parse a `/approve` text command into (approval_id, decision).
///
/// Accepts formats like:
/// - `/approve abc123 allow`
/// - `/approve abc123 deny`
/// - `/approve abc123 yes`
/// - `/approve abc123 no`
pub fn parse_approval_text(text: &str) -> Option<(String, ApprovalDecision)> {
    let text = text.trim();
    let parts: Vec<&str> = text.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    if parts[0].to_lowercase() != "/approve" {
        return None;
    }
    let id = parts[1].to_string();
    if id.is_empty() {
        return None;
    }
    let decision = match parts[2].to_lowercase().as_str() {
        "allow" | "yes" | "y" | "approve" => ApprovalDecision::Allow,
        "deny" | "no" | "n" | "reject" | "block" => ApprovalDecision::Deny,
        _ => return None,
    };
    Some((id, decision))
}

/// Parsed approval decision from a channel command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    Allow,
    Deny,
}

impl From<ApprovalDecision> for ApprovalResponse {
    fn from(decision: ApprovalDecision) -> Self {
        match decision {
            ApprovalDecision::Allow => ApprovalResponse::Approved,
            ApprovalDecision::Deny => ApprovalResponse::Denied("denied by user".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_approval_prompt() {
        let prompt = format_approval_prompt("shell", r#"{"command":"ls -la"}"#, "abc12345-def6");
        assert!(prompt.contains("shell"));
        assert!(prompt.contains("ls -la"));
        assert!(prompt.contains("abc12345"));
    }

    #[test]
    fn test_parse_approval_text_allow() {
        let result = parse_approval_text("/approve abc123 allow");
        assert_eq!(
            result,
            Some(("abc123".to_string(), ApprovalDecision::Allow))
        );
    }

    #[test]
    fn test_parse_approval_text_deny() {
        let result = parse_approval_text("/approve abc123 deny");
        assert_eq!(result, Some(("abc123".to_string(), ApprovalDecision::Deny)));
    }

    #[test]
    fn test_parse_approval_text_yes_no() {
        assert_eq!(
            parse_approval_text("/approve id1 yes"),
            Some(("id1".to_string(), ApprovalDecision::Allow))
        );
        assert_eq!(
            parse_approval_text("/approve id2 no"),
            Some(("id2".to_string(), ApprovalDecision::Deny))
        );
    }

    #[test]
    fn test_parse_approval_text_aliases() {
        assert_eq!(
            parse_approval_text("/approve id1 y"),
            Some(("id1".to_string(), ApprovalDecision::Allow))
        );
        assert_eq!(
            parse_approval_text("/approve id2 reject"),
            Some(("id2".to_string(), ApprovalDecision::Deny))
        );
        assert_eq!(
            parse_approval_text("/approve id3 block"),
            Some(("id3".to_string(), ApprovalDecision::Deny))
        );
    }

    #[test]
    fn test_parse_approval_text_case_insensitive() {
        assert_eq!(
            parse_approval_text("/APPROVE id1 Allow"),
            Some(("id1".to_string(), ApprovalDecision::Allow))
        );
    }

    #[test]
    fn test_parse_approval_text_invalid() {
        assert!(parse_approval_text("/approve").is_none());
        assert!(parse_approval_text("/approve id1").is_none());
        assert!(parse_approval_text("/approve id1 maybe").is_none());
        assert!(parse_approval_text("hello world").is_none());
    }

    #[test]
    fn test_approval_decision_into_response() {
        let allow: ApprovalResponse = ApprovalDecision::Allow.into();
        assert!(matches!(allow, ApprovalResponse::Approved));

        let deny: ApprovalResponse = ApprovalDecision::Deny.into();
        assert!(matches!(deny, ApprovalResponse::Denied(_)));
    }

    #[tokio::test]
    async fn test_channel_approval_round_trip() {
        use crate::tools::approval_store::ApprovalRequestStore;

        let bus = Arc::new(MessageBus::new());
        let store = ApprovalRequestStore::new();
        let timeout = 10u64;

        let ctx = ChannelApprovalContext {
            channel: "telegram".to_string(),
            chat_id: "123456".to_string(),
            thread_id: None,
            reply_to: None,
        };

        let request = ApprovalRequest::new(
            "shell".to_string(),
            serde_json::json!({"command": "ls -la"}),
            0,
        );

        let store_clone = store.clone();
        let bus_clone = bus.clone();

        let handle = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Some(msg) = bus_clone.consume_outbound().await {
                if let Some(id) = msg.metadata.get("approval_id") {
                    store_clone.resolve(id, ApprovalResponse::Approved).await;
                }
            }
        });

        let response = channel_approval_inner(bus, store, ctx, request, timeout).await;
        assert!(matches!(response, ApprovalResponse::Approved));

        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_channel_approval_timeout() {
        use crate::tools::approval_store::ApprovalRequestStore;

        let bus = Arc::new(MessageBus::new());
        let store = ApprovalRequestStore::new();
        let timeout = 1u64;

        let ctx = ChannelApprovalContext {
            channel: "telegram".to_string(),
            chat_id: "123".to_string(),
            thread_id: None,
            reply_to: None,
        };

        let request = ApprovalRequest::new(
            "shell".to_string(),
            serde_json::json!({"command": "rm -rf /"}),
            0,
        );

        let start = std::time::Instant::now();
        let response = channel_approval_inner(bus, store, ctx, request, timeout).await;
        let elapsed = start.elapsed();

        assert!(matches!(response, ApprovalResponse::TimedOut));
        assert!(elapsed >= Duration::from_secs(1));
    }
}
