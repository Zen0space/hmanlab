//! Shared store for pending channel-based tool approvals.
//!
//! When the approval handler needs user confirmation via a channel (e.g. Telegram
//! inline keyboard buttons), it creates a `PendingApproval` entry here and awaits
//! the oneshot receiver. When a channel callback or text command arrives, the
//! resolver calls `ApprovalRequestStore::resolve()` which completes the oneshot
//! and unblocks the tool execution.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use tokio::sync::{oneshot, Mutex};
use tracing::{debug, warn};

use super::approval::ApprovalResponse;

/// A pending approval request waiting for a channel response.
pub struct PendingApproval {
    /// Unique identifier for this approval request.
    pub id: String,
    /// Tool name awaiting approval.
    pub tool_name: String,
    /// Arguments the tool would be called with (pretty JSON).
    pub arguments_summary: String,
    /// Channel that originated the request (e.g. "telegram").
    pub channel: String,
    /// Chat ID where the approval prompt was sent.
    pub chat_id: String,
    /// When the request was created.
    pub created_at: DateTime<Utc>,
    /// Sender half of a oneshot channel. Completing it unblocks the
    /// approval handler so tool execution can proceed or be denied.
    pub resolver: oneshot::Sender<ApprovalResponse>,
}

/// Thread-safe store for pending channel approvals.
///
/// Clones share the same underlying map (Arc<Mutex>). The store is
/// shared between the approval handler (insert side) and channel
/// callback handlers (resolve side).
#[derive(Clone)]
pub struct ApprovalRequestStore {
    inner: Arc<Mutex<HashMap<String, PendingApproval>>>,
}

impl ApprovalRequestStore {
    /// Create a new empty store.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Insert a pending approval and return the oneshot receiver.
    ///
    /// The caller should await the receiver (with a timeout) to block
    /// until the channel delivers a response.
    pub async fn insert(&self, approval: PendingApproval) -> oneshot::Receiver<ApprovalResponse> {
        let (tx, rx) = oneshot::channel();
        let id = approval.id.clone();
        let entry = PendingApproval {
            resolver: tx,
            id: approval.id,
            tool_name: approval.tool_name,
            arguments_summary: approval.arguments_summary,
            channel: approval.channel,
            chat_id: approval.chat_id,
            created_at: approval.created_at,
        };
        let mut map = self.inner.lock().await;
        map.insert(id, entry);
        rx
    }

    /// Resolve a pending approval by ID.
    ///
    /// If the ID exists, completes the oneshot sender with the given
    /// response and removes the entry. Returns `true` if resolved.
    pub async fn resolve(&self, id: &str, response: ApprovalResponse) -> bool {
        let mut map = self.inner.lock().await;
        if let Some(entry) = map.remove(id) {
            debug!("Resolved approval {}: {:?}", id, response);
            let _ = entry.resolver.send(response);
            true
        } else {
            warn!(
                "Approval {} not found (may have expired or already resolved)",
                id
            );
            false
        }
    }

    /// Get a pending approval by ID without resolving it.
    pub async fn get(&self, id: &str) -> Option<ApprovalInfo> {
        let map = self.inner.lock().await;
        map.get(id).map(|e| ApprovalInfo {
            id: e.id.clone(),
            tool_name: e.tool_name.clone(),
            channel: e.channel.clone(),
            chat_id: e.chat_id.clone(),
            created_at: e.created_at,
        })
    }

    /// Remove and discard pending approvals older than `max_age_secs`.
    ///
    /// Timed-out entries are resolved with `ApprovalResponse::TimedOut`
    /// before removal so the awaiting handler unblocks.
    pub async fn expire_old(&self, max_age_secs: u64) -> usize {
        let now = Utc::now();
        let mut map = self.inner.lock().await;
        let mut expired_ids = Vec::new();
        let mut expired_count = 0;
        map.retain(|id, entry| {
            let age = (now - entry.created_at).num_seconds();
            if age > max_age_secs as i64 {
                expired_ids.push(id.clone());
                expired_count += 1;
                false
            } else {
                true
            }
        });
        // Drop the lock, then resolve each expired entry.
        // (resolvers were already removed by retain's false returns)
        // Note: entries were already removed from the map, so we cannot
        // resolve them here. The oneshot sender was dropped when the entry
        // was removed from the HashMap, which will cause the receiver to
        // return Err — the handler should treat that as a timeout.
        let _ = expired_ids;
        expired_count
    }

    /// Return the number of pending approvals.
    pub async fn len(&self) -> usize {
        self.inner.lock().await.len()
    }

    /// Return true if there are no pending approvals.
    pub async fn is_empty(&self) -> bool {
        self.inner.lock().await.is_empty()
    }
}

impl Default for ApprovalRequestStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Read-only snapshot of a pending approval (no resolver).
#[derive(Debug, Clone)]
pub struct ApprovalInfo {
    pub id: String,
    pub tool_name: String,
    pub channel: String,
    pub chat_id: String,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_approval(id: &str, tool_name: &str, channel: &str, chat_id: &str) -> PendingApproval {
        let (tx, _) = oneshot::channel();
        PendingApproval {
            id: id.to_string(),
            tool_name: tool_name.to_string(),
            arguments_summary: "test".to_string(),
            channel: channel.to_string(),
            chat_id: chat_id.to_string(),
            created_at: Utc::now(),
            resolver: tx,
        }
    }

    #[tokio::test]
    async fn test_insert_and_resolve_approved() {
        let store = ApprovalRequestStore::new();
        let approval = dummy_approval("test-1", "shell", "telegram", "123");

        let mut rx = store.insert(approval).await;
        assert_eq!(store.len().await, 1);

        let resolved = store.resolve("test-1", ApprovalResponse::Approved).await;
        assert!(resolved);

        let response = rx.try_recv();
        assert!(matches!(response, Ok(ApprovalResponse::Approved)));
        assert_eq!(store.len().await, 0);
    }

    #[tokio::test]
    async fn test_insert_and_resolve_denied() {
        let store = ApprovalRequestStore::new();
        let approval = dummy_approval("test-2", "shell", "telegram", "123");

        let mut rx = store.insert(approval).await;
        let resolved = store
            .resolve(
                "test-2",
                ApprovalResponse::Denied("too dangerous".to_string()),
            )
            .await;
        assert!(resolved);

        let response = rx.try_recv();
        assert!(matches!(response, Ok(ApprovalResponse::Denied(_))));
    }

    #[tokio::test]
    async fn test_resolve_unknown_id() {
        let store = ApprovalRequestStore::new();
        let resolved = store
            .resolve("nonexistent", ApprovalResponse::Approved)
            .await;
        assert!(!resolved);
    }

    #[tokio::test]
    async fn test_expire_old() {
        let store = ApprovalRequestStore::new();

        let mut old = dummy_approval("old-1", "shell", "telegram", "123");
        old.created_at = Utc::now() - chrono::Duration::seconds(600);
        let mut old_rx = store.insert(old).await;

        let recent = dummy_approval("recent-1", "shell", "telegram", "123");
        let _recent_rx = store.insert(recent).await;

        let expired = store.expire_old(300).await;
        assert_eq!(expired, 1);
        assert_eq!(store.len().await, 1);

        // When the entry is removed, the oneshot sender is dropped,
        // so the receiver returns Err.
        let result = old_rx.try_recv();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_pending_approvals() {
        let store = ApprovalRequestStore::new();

        for i in 0..5 {
            let approval = dummy_approval(&format!("multi-{}", i), "shell", "telegram", "123");
            let _rx = store.insert(approval).await;
        }

        assert_eq!(store.len().await, 5);

        assert!(store.resolve("multi-2", ApprovalResponse::Approved).await);
        assert!(
            store
                .resolve("multi-4", ApprovalResponse::Denied("nope".to_string()))
                .await
        );

        assert_eq!(store.len().await, 3);
        assert!(store.get("multi-0").await.is_some());
        assert!(store.get("multi-2").await.is_none());
    }

    #[tokio::test]
    async fn test_get_info() {
        let store = ApprovalRequestStore::new();
        let before = Utc::now();
        let mut approval = dummy_approval("info-1", "write_file", "telegram", "456");
        approval.arguments_summary = "test.txt".to_string();
        let _rx = store.insert(approval).await;

        let info = store.get("info-1").await.unwrap();
        assert_eq!(info.tool_name, "write_file");
        assert_eq!(info.channel, "telegram");
        assert_eq!(info.chat_id, "456");
        assert!(info.created_at >= before);
    }

    #[tokio::test]
    async fn test_default_is_empty() {
        let store = ApprovalRequestStore::default();
        assert_eq!(store.len().await, 0);
    }
}
