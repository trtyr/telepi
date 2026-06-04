use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::config::TelePiConfig;
use crate::error::Result;
use crate::pi::session::{SessionContext, SessionInfo, PiSession};
use crate::pi::cli_session::CliSession;

/// Manages per-chat Pi sessions.
///
/// Each Telegram chat (identified by `SessionContext`) gets its own
/// `PiSession` wrapping a Pi agent instance.
#[derive(Clone)]
pub struct SessionRegistry {
    inner: Arc<RwLock<SessionRegistryInner>>,
    config: Arc<TelePiConfig>,
}

impl std::fmt::Debug for SessionRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionRegistry").finish()
    }
}

struct SessionRegistryInner {
    sessions: HashMap<SessionContext, Arc<dyn PiSession>>,
    bootstrap_session_path: Option<std::path::PathBuf>,
}

impl SessionRegistry {
    pub fn new(config: Arc<TelePiConfig>) -> Self {
        let bootstrap = config.pi_session_path.clone();
        Self {
            inner: Arc::new(RwLock::new(SessionRegistryInner {
                sessions: HashMap::new(),
                bootstrap_session_path: bootstrap,
            })),
            config,
        }
    }

    /// Get an existing session or create a new one for the given context.
    pub async fn get_or_create(&self, ctx: &SessionContext) -> Result<Arc<dyn PiSession>> {
        // Check if session already exists
        {
            let inner = self.inner.read().await;
            if let Some(session) = inner.sessions.get(ctx) {
                return Ok(session.clone());
            }
        }

        // Create a new session
        let mut inner = self.inner.write().await;

        // Double-check after acquiring write lock
        if let Some(session) = inner.sessions.get(ctx) {
            return Ok(session.clone());
        }

        // Consume bootstrap path on first call
        let bootstrap = inner.bootstrap_session_path.take();

        let session = CliSession::create(
            self.config.clone(),
            ctx.clone(),
            bootstrap,
        ).await?;

        let session: Arc<dyn PiSession> = Arc::new(session);
        inner.sessions.insert(ctx.clone(), session.clone());

        Ok(session)
    }

    /// Remove a session for the given context.
    pub async fn remove(&self, ctx: &SessionContext) {
        let mut inner = self.inner.write().await;
        if let Some(session) = inner.sessions.remove(ctx) {
            session.dispose().await.ok();
        }
    }

    /// List all active sessions.
    pub async fn list(&self) -> Vec<SessionInfo> {
        let inner = self.inner.read().await;
        inner.sessions.values().map(|s| s.info()).collect()
    }
}
