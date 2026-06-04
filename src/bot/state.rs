use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::pi::session::SessionContext;

/// Per-chat busy state tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatStatus {
    Idle,
    Processing,
    Switching,
    Transcribing,
}

/// Key identifying a unique Telegram conversation context.
/// Maps to a single Pi session.
pub type ChatKey = String;

/// Build a chat key from chat ID and optional message thread ID.
pub fn chat_key(chat_id: i64, message_thread_id: Option<teloxide::types::ThreadId>) -> ChatKey {
    match message_thread_id {
        Some(tid) => format!("{chat_id}::{}", tid.0),
        None => chat_id.to_string(),
    }
}

/// Convert a chat key to a session context.
pub fn chat_key_to_context(key: &ChatKey) -> SessionContext {
    if let Some((chat_str, thread_str)) = key.split_once("::") {
        SessionContext {
            chat_id: chat_str.parse().unwrap_or(0),
            message_thread_id: thread_str.parse().ok(),
        }
    } else {
        SessionContext {
            chat_id: key.parse().unwrap_or(0),
            message_thread_id: None,
        }
    }
}

/// Thread-safe per-chat state tracker.
#[derive(Debug, Clone)]
pub struct BotChatState {
    inner: Arc<Mutex<BotChatStateInner>>,
}

#[derive(Debug, Default)]
struct BotChatStateInner {
    statuses: HashMap<ChatKey, ChatStatus>,
    last_prompts: HashMap<ChatKey, String>,
}

impl BotChatState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BotChatStateInner::default())),
        }
    }

    pub async fn status(&self, key: &ChatKey) -> ChatStatus {
        let inner = self.inner.lock().await;
        *inner.statuses.get(key).unwrap_or(&ChatStatus::Idle)
    }

    pub async fn is_busy(&self, key: &ChatKey) -> bool {
        self.status(key).await != ChatStatus::Idle
    }

    pub async fn begin_processing(&self, key: &ChatKey, prompt: &str) {
        let mut inner = self.inner.lock().await;
        inner.statuses.insert(key.clone(), ChatStatus::Processing);
        inner.last_prompts.insert(key.clone(), prompt.to_string());
    }

    pub async fn end_processing(&self, key: &ChatKey) {
        let mut inner = self.inner.lock().await;
        inner.statuses.remove(key);
    }

    pub async fn begin_switching(&self, key: &ChatKey) {
        let mut inner = self.inner.lock().await;
        inner.statuses.insert(key.clone(), ChatStatus::Switching);
    }

    pub async fn end_switching(&self, key: &ChatKey) {
        let mut inner = self.inner.lock().await;
        inner.statuses.remove(key);
    }

    pub async fn begin_transcribing(&self, key: &ChatKey) {
        let mut inner = self.inner.lock().await;
        inner.statuses.insert(key.clone(), ChatStatus::Transcribing);
    }

    pub async fn end_transcribing(&self, key: &ChatKey) {
        let mut inner = self.inner.lock().await;
        inner.statuses.remove(key);
    }

    pub async fn last_prompt(&self, key: &ChatKey) -> Option<String> {
        let inner = self.inner.lock().await;
        inner.last_prompts.get(key).cloned()
    }
}
