use std::collections::HashMap;
use std::sync::Arc;

use teloxide::prelude::*;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::bot::state::{self, BotChatState, ChatKey};
use crate::bot::transport;
use crate::config::TelePiConfig;
use crate::format::escape_html;
use crate::pi::registry::SessionRegistry;
use crate::error::to_friendly_error;
use crate::pi::session::PiEvent;

/// Shared state passed to all handlers.
#[derive(Clone)]
pub struct HandlerState {
    pub config: Arc<TelePiConfig>,
    pub sessions: SessionRegistry,
    pub chat_state: BotChatState,
    /// Shared HTTP client with timeouts and proxy configured.
    pub http: reqwest::Client,
    /// Model lists cached per chat for callback handling.
    pub model_lists: Arc<tokio::sync::Mutex<HashMap<ChatKey, Vec<crate::pi::cli_session::ModelInfo>>>>,
}

impl HandlerState {
    pub async fn set_model_list(
        &self,
        chat_id: teloxide::types::ChatId,
        thread_id: Option<teloxide::types::ThreadId>,
        models: Vec<crate::pi::cli_session::ModelInfo>,
    ) {
        let key = state::chat_key(chat_id.0, thread_id);
        let mut lists = self.model_lists.lock().await;
        lists.insert(key, models);
    }

    pub async fn get_model_list(
        &self,
        chat_id: teloxide::types::ChatId,
        thread_id: Option<teloxide::types::ThreadId>,
    ) -> Option<Vec<crate::pi::cli_session::ModelInfo>> {
        let key = state::chat_key(chat_id.0, thread_id);
        let lists = self.model_lists.lock().await;
        lists.get(&key).cloned()
    }
}

/// Teloxide endpoint: handle plain text messages (not commands).
pub async fn text_handler(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
    let Some(text) = msg.text() else {
        return Ok(());
    };
    let Some(user) = &msg.from else {
        return Ok(());
    };

    if !state.config.is_allowed_user(user.id.0) {
        warn!(user_id = user.id.0, "rejected unauthorized user");
        bot.send_message(msg.chat.id, "⛔ You are not authorized to use this bot.")
            .await?;
        return Ok(());
    }

    let key = state::chat_key(msg.chat.id.0, msg.thread_id.clone());

    if state.chat_state.is_busy(&key).await {
        bot.send_message(msg.chat.id, "⏳ Still processing the previous prompt. Use /abort to cancel.")
            .await?;
        return Ok(());
    }

    let prompt_text = text.to_string();
    info!(chat_key = %key, prompt_len = prompt_text.len(), "received text prompt");

    state.chat_state.begin_processing(&key, &prompt_text).await;
    transport::send_typing(&bot, msg.chat.id).await.ok();

    let result = process_prompt(&bot, &msg, &state, &key, &prompt_text).await;
    state.chat_state.end_processing(&key).await;

    if let Err(e) = result {
        error!(error = %e, "failed to process prompt");
        bot.send_message(msg.chat.id, format!("❌ {}", to_friendly_error(&e)))
            .await
            .ok();
    }

    Ok(())
}

/// Download a URL with retry and exponential backoff.
async fn download_with_retry(client: &reqwest::Client, url: &str) -> crate::error::Result<Vec<u8>> {
    const MAX_RETRIES: u32 = 3;
    const BASE_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

    let mut last_err = None;
    for attempt in 0..MAX_RETRIES {
        match client.get(url).send().await {
            Ok(resp) => match resp.bytes().await {
                Ok(b) => return Ok(b.to_vec()),
                Err(e) => {
                    last_err = Some(e.into());
                }
            },
            Err(e) => {
                last_err = Some(e.into());
            }
        }

        if attempt < MAX_RETRIES - 1 {
            let delay = BASE_DELAY * (attempt + 1);
            tracing::warn!(attempt = attempt + 1, delay = ?delay, "download failed, retrying...");
            tokio::time::sleep(delay).await;
        }
    }

    Err(last_err.unwrap())
}

/// Teloxide endpoint: handle voice/audio messages.
pub async fn voice_handler(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
    let Some(user) = &msg.from else {
        return Ok(());
    };
    if !state.config.is_allowed_user(user.id.0) {
        return Ok(());
    }

    let key = state::chat_key(msg.chat.id.0, msg.thread_id.clone());

    if state.chat_state.is_busy(&key).await {
        bot.send_message(msg.chat.id, "⏳ Still processing. Use /abort to cancel.")
            .await?;
        return Ok(());
    }

    state.chat_state.begin_transcribing(&key).await;

    let status_msg = bot.send_message(msg.chat.id, "🎙️ Downloading voice message...")
        .await?;

    // Download the voice/audio file
    let file_id = if let Some(v) = msg.voice() {
        v.file.id.clone()
    } else if let Some(a) = msg.audio() {
        a.file.id.clone()
    } else {
        state.chat_state.end_transcribing(&key).await;
        transport::edit_text(&bot, msg.chat.id, status_msg.id, "❌ No voice data found.").await.ok();
        return Ok(());
    };

    // Download from Telegram
    let temp_dir = std::env::temp_dir().join("telepi");
    tokio::fs::create_dir_all(&temp_dir).await.ok();
    let ogg_path = temp_dir.join(format!("voice_{}.ogg", msg.id));
    let tg_file = bot.get_file(&file_id).await?;
    let path = tg_file.path;
    let url = format!("https://api.telegram.org/file/bot{}/{}", &state.config.telegram_bot_token, path);

    let resp = match download_with_retry(&state.http, &url).await {
        Ok(b) => b,
        Err(e) => {
            state.chat_state.end_transcribing(&key).await;
            transport::edit_text(&bot, msg.chat.id, status_msg.id, &format!("❌ Download failed: {e}")).await.ok();
            return Ok(());
        }
    };
    if let Err(e) = tokio::fs::write(&ogg_path, &resp).await {
        state.chat_state.end_transcribing(&key).await;
        transport::edit_text(&bot, msg.chat.id, status_msg.id, &format!("❌ File write failed: {e}")).await.ok();
        return Ok(());
    }

    info!(path = %ogg_path.display(), "voice file downloaded");

    // Transcribe
    transport::edit_text(&bot, msg.chat.id, status_msg.id, "🎙️ Transcribing...").await.ok();

    match crate::voice::transcribe(&ogg_path).await {
        Ok(transcript) => {
            info!(len = transcript.text.len(), "transcription complete");
            let preview = if transcript.text.chars().count() > 100 {
                format!("{}...", transcript.text.chars().take(100).collect::<String>())
            } else {
                transcript.text.clone()
            };
            transport::edit_text(
                &bot, msg.chat.id, status_msg.id,
                &format!("🎙️ <b>Transcribed:</b>\n{}", escape_html(&preview))
            ).await.ok();

            // Now process the transcript as a prompt
            state.chat_state.end_transcribing(&key).await;
            state.chat_state.begin_processing(&key, &transcript.text).await;
            transport::send_typing(&bot, msg.chat.id).await.ok();

            let result = process_prompt(&bot, &msg, &state, &key, &transcript.text).await;
            state.chat_state.end_processing(&key).await;

            if let Err(e) = result {
                error!(error = %e, "failed to process voice prompt");
                bot.send_message(msg.chat.id, format!("❌ {}", to_friendly_error(&e)))
                    .await.ok();
            }
        }
        Err(e) => {
            error!(error = %e, "transcription failed");
            state.chat_state.end_transcribing(&key).await;
            transport::edit_text(
                &bot, msg.chat.id, status_msg.id,
                &format!("❌ Transcription failed: {e}")
            ).await.ok();
        }
    }

    // Cleanup temp file
    tokio::fs::remove_file(&ogg_path).await.ok();

    Ok(())
}

/// Teloxide endpoint: handle photo/document messages.
pub async fn photo_handler(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
    let Some(user) = &msg.from else {
        return Ok(());
    };
    if !state.config.is_allowed_user(user.id.0) {
        return Ok(());
    };

    let key = state::chat_key(msg.chat.id.0, msg.thread_id.clone());

    if state.chat_state.is_busy(&key).await {
        bot.send_message(msg.chat.id, "⏳ Still processing. Use /abort to cancel.")
            .await?;
        return Ok(());
    }

    // Get the caption as prompt text, or use a default
    let caption = msg.caption().unwrap_or("What's in this image?").to_string();

    // Download the photo (largest available)
    let photo = match msg.photo() {
        Some(photos) => photos.last(),
        None => None,
    };

    let file_id = match photo {
        Some(p) => p.file.id.clone(),
        None => {
            // Try document
            match msg.document() {
                Some(doc) => doc.file.id.clone(),
                None => {
                    bot.send_message(msg.chat.id, "❌ No image found.").await?;
                    return Ok(());
                }
            }
        }
    };

    let status_msg = bot.send_message(msg.chat.id, "📸 Downloading image...").await?;

    // Download from Telegram
    let temp_dir = std::env::temp_dir().join("telepi");
    tokio::fs::create_dir_all(&temp_dir).await.ok();
    let img_path = temp_dir.join(format!("photo_{}.jpg", msg.id));
    let tg_file = bot.get_file(&file_id).await?;
    let path = tg_file.path;
    let url = format!("https://api.telegram.org/file/bot{}/{}", &state.config.telegram_bot_token, path);

    let resp = match download_with_retry(&state.http, &url).await {
        Ok(b) => b,
        Err(e) => {
            state.chat_state.end_processing(&key).await;
            transport::edit_text(&bot, msg.chat.id, status_msg.id, &format!("❌ Download failed: {e}")).await.ok();
            return Ok(());
        }
    };
    if let Err(e) = tokio::fs::write(&img_path, &resp).await {
        state.chat_state.end_processing(&key).await;
        transport::edit_text(&bot, msg.chat.id, status_msg.id, &format!("❌ File write failed: {e}")).await.ok();
        return Ok(());
    };

    info!(path = %img_path.display(), "photo file downloaded");

    state.chat_state.begin_processing(&key, &caption).await;
    transport::edit_text(&bot, msg.chat.id, status_msg.id, "📸 Processing image...").await.ok();
    transport::send_typing(&bot, msg.chat.id).await.ok();

    // Process with image
    let ctx = state::chat_key_to_context(&key);
    let session = match state.sessions.get_or_create(&ctx).await {
        Ok(s) => s,
        Err(e) => {
            state.chat_state.end_processing(&key).await;
            transport::edit_text(&bot, msg.chat.id, status_msg.id, &format!("❌ Session error: {e}")).await.ok();
            return Ok(());
        }
    };

    let thinking_msg = bot.send_message(msg.chat.id, "🤔 Thinking...").await?;

    let result = session.prompt_with_images(&caption, &[img_path.clone()]).await;


    // Cleanup temp file
    tokio::fs::remove_file(&img_path).await.ok();

    state.chat_state.end_processing(&key).await;

    match result {
        Ok(response) => {
            let formatted = format!("<b>Pi:</b>\n{}", escape_html(&response.text));
            transport::edit_text(&bot, msg.chat.id, thinking_msg.id, &formatted).await?;
        }
        Err(e) => {
            error!(error = %e, "failed to process image prompt");
            bot.send_message(msg.chat.id, format!("❌ {}", to_friendly_error(&e)))
                .await.ok();
        }
    }

    Ok(())
}

/// Process a text prompt through the Pi session with streaming.
async fn process_prompt(
    bot: &Bot,
    msg: &Message,
    state: &HandlerState,
    key: &ChatKey,
    prompt: &str,
) -> crate::error::Result<()> {
    let ctx = state::chat_key_to_context(key);
    let session = state.sessions.get_or_create(&ctx).await?;

    // Send initial message
    let thinking_msg = bot
        .send_message(msg.chat.id, "🤔 Thinking...")
        .await?;

    // Create event channel
    let (tx, mut rx) = mpsc::channel::<PiEvent>(256);

    // Spawn a task to handle streaming edits
    let bot_clone = bot.clone();
    let chat_id = msg.chat.id;
    let msg_id = thinking_msg.id;
    let edit_task = tokio::spawn(async move {
        let mut accumulated = String::new();
        let mut last_edit_time = std::time::Instant::now();
        let mut last_edit_text = String::new();
        let debounce_ms = 1500; // Edit at most every 1.5s
        let mut _current_tool: Option<String> = None;
        let mut tool_output_lines: Vec<String> = Vec::new();

        while let Some(event) = rx.recv().await {
            match event {
                PiEvent::ThinkingDelta { .. } => {
                    // Don't show thinking in Telegram — too noisy
                }
                PiEvent::TextDelta { delta } => {
                    accumulated.push_str(&delta);

                    // Debounced editing: only edit if enough time has passed
                    let now = std::time::Instant::now();
                    if now.duration_since(last_edit_time).as_millis() >= debounce_ms {
                        let display = format!(
                            "<b>Pi:</b> 🔄\n{}",
                            escape_html(&accumulated)
                        );
                        // Only edit if text actually changed
                        if display != last_edit_text {
                            transport::edit_text(&bot_clone, chat_id, msg_id, &display).await.ok();
                            last_edit_text = display;
                            last_edit_time = now;
                        }
                    }
                }
                PiEvent::ToolStart { tool_name, .. } => {
                    _current_tool = Some(tool_name.clone());
                    tool_output_lines.clear();

                    // Show tool call indicator
                    let indicator = if accumulated.is_empty() {
                        format!("🔧 <i>{}</i>...", escape_html(&tool_name))
                    } else {
                        format!(
                            "<b>Pi:</b> 🔄\n{}\n\n🔧 <i>{}</i>...",
                            escape_html(&accumulated),
                            escape_html(&tool_name)
                        )
                    };
                    transport::edit_text(&bot_clone, chat_id, msg_id, &indicator).await.ok();
                    last_edit_text = indicator;
                    last_edit_time = std::time::Instant::now();
                }
                PiEvent::ToolEnd { .. } => {
                    _current_tool = None;
                    tool_output_lines.clear();

                    // Restore to accumulated text
                    let display = format!(
                        "<b>Pi:</b> 🔄\n{}",
                        escape_html(&accumulated)
                    );
                    transport::edit_text(&bot_clone, chat_id, msg_id, &display).await.ok();
                    last_edit_text = display;
                    last_edit_time = std::time::Instant::now();
                }
                PiEvent::ToolOutput { output, .. } => {
                    tool_output_lines.push(output);
                }
                PiEvent::Usage { tokens_in, tokens_out, cost, .. } => {
                    // Could store this for /context command
                    let _ = (tokens_in, tokens_out, cost);
                }
                PiEvent::TurnEnd { text } => {
                    // Final edit with complete text
                    let formatted = format!("<b>Pi:</b>\n{}", escape_html(&text));
                    transport::edit_text(&bot_clone, chat_id, msg_id, &formatted).await.ok();
                }
                PiEvent::Error { message } => {
                    let err_text = format!(
                        "<b>Pi:</b>\n{}\n\n❌ <i>{}</i>",
                        escape_html(&accumulated),
                        escape_html(&message)
                    );
                    transport::edit_text(&bot_clone, chat_id, msg_id, &err_text).await.ok();
                }
            }
        }
    });

    // Run the streaming prompt
    let result = session.prompt_streaming(prompt, tx).await;

    // Wait for the edit task to finish processing remaining events
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        edit_task,
    ).await;

    result?;

    Ok(())
}

/// Teloxide endpoint: handle /abort command.
pub async fn abort_handler(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
    let key = state::chat_key(msg.chat.id.0, msg.thread_id.clone());

    if !state.chat_state.is_busy(&key).await {
        bot.send_message(msg.chat.id, "Nothing to abort.").await?;
        return Ok(());
    }

    // Get the session and abort it
    let ctx = state::chat_key_to_context(&key);
    match state.sessions.get_or_create(&ctx).await {
        Ok(session) => {
            if let Err(e) = session.abort().await {
                warn!(error = %e, "abort failed");
            }
        }
        Err(e) => {
            warn!(error = %e, "could not get session for abort");
        }
    }

    state.chat_state.end_processing(&key).await;
    bot.send_message(msg.chat.id, "🛑 Aborted.").await?;

    Ok(())
}

/// Teloxide endpoint: handle /retry command.
pub async fn retry_handler(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
    let key = state::chat_key(msg.chat.id.0, msg.thread_id.clone());

    let Some(last) = state.chat_state.last_prompt(&key).await else {
        bot.send_message(msg.chat.id, "No previous prompt to retry.").await?;
        return Ok(());
    };

    info!(chat_key = %key, "retrying last prompt");

    state.chat_state.begin_processing(&key, &last).await;
    let result = process_prompt(&bot, &msg, &state, &key, &last).await;
    state.chat_state.end_processing(&key).await;

    if let Err(e) = result {
        error!(error = %e, "retry failed");
        bot.send_message(msg.chat.id, format!("❌ {}", to_friendly_error(&e)))
            .await
            .ok();
    }

    Ok(())
}
