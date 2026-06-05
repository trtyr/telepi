use std::path::PathBuf;
use std::sync::Arc;

use tokio::time::{interval, Duration};
use tracing::{error, info};

use crate::bot::handler::HandlerState;
use crate::bot::state;
use crate::config::TelePiConfig;

/// Start polling the prompt inbox directory for .txt files.
pub fn start_prompt_inbox_polling(
    config: Arc<TelePiConfig>,
    state: HandlerState,
) -> Option<tokio::task::JoinHandle<()>> {
    let inbox_dir = config.prompt_inbox_dir.clone()?;
    let interval_ms = config.prompt_inbox_interval_ms;

    if interval_ms == 0 {
        return None;
    }

    info!(
        dir = %inbox_dir.display(),
        interval_ms = interval_ms,
        "starting prompt inbox polling"
    );

    let handle = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(interval_ms));
        ticker.tick().await;

        loop {
            ticker.tick().await;
            if let Err(e) = poll_inbox_once(&inbox_dir, &state).await {
                error!(error = %e, "prompt inbox: poll failed");
            }
        }
    });

    Some(handle)
}

async fn poll_inbox_once(inbox_dir: &PathBuf, state: &HandlerState) -> crate::error::Result<bool> {
    if !tokio::fs::try_exists(inbox_dir).await.unwrap_or(false) {
        return Ok(false);
    }

    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(inbox_dir).await?;

    while let Some(entry) = dir.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("txt") {
            if let Ok(meta) = entry.metadata().await {
                if meta.len() > 0 {
                    entries.push((path, meta.modified().unwrap_or(std::time::UNIX_EPOCH)));
                }
            }
        }
    }

    if entries.is_empty() {
        return Ok(false);
    }

    entries.sort_by_key(|(_, mtime)| *mtime);
    let (path, _) = &entries[0];

    let content = tokio::fs::read_to_string(path).await?;
    let content = content.trim().to_string();

    if content.is_empty() {
        tokio::fs::remove_file(path).await?;
        return Ok(false);
    }

    let default_key = "inbox".to_string();
    if state.chat_state.is_busy(&default_key).await {
        return Ok(false);
    }

    info!(file = %path.display(), "processing inbox prompt");

    state.chat_state.begin_processing(&default_key, &content).await;

    let ctx = state::chat_key_to_context(&default_key);
    let session = match state.sessions.get_or_create(&ctx).await {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "failed to get session for inbox");
            state.chat_state.end_processing(&default_key).await;
            tokio::fs::remove_file(path).await?;
            return Ok(false);
        }
    };

    match session.prompt(&content).await {
        Ok(_) => info!("inbox prompt completed"),
        Err(e) => error!(error = %e, "inbox prompt failed"),
    }

    state.chat_state.end_processing(&default_key).await;
    tokio::fs::remove_file(path).await?;

    Ok(true)
}
