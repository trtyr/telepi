use teloxide::prelude::*;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

use crate::bot::handler::HandlerState;
use crate::pi::cli_session::CliSession;

/// /model — Show or switch the AI model with inline keyboard picker
pub async fn cmd_model(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
    let key = crate::bot::state::chat_key(msg.chat.id.0, msg.thread_id.clone());
    let ctx = crate::bot::state::chat_key_to_context(&key);

    let session = match state.sessions.get_or_create(&ctx).await {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(msg.chat.id, format!("❌ Session error: {e}"))
                .await?;
            return Ok(());
        }
    };

    let info = session.info();
    let current_model = info.model.clone().unwrap_or_else(|| "default".to_string());

    // Try to list available models
    match CliSession::list_models().await {
        Ok(models) if !models.is_empty() => {
            let text = format!(
                "🤖 <b>Current model:</b> <code>{}</code>\n\nSelect a model to switch:\n",
                current_model
            );

            let mut keyboard_rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();

            for (i, model) in models.iter().take(20).enumerate() {
                let label = if format!("{}/{}", model.provider, model.model) == current_model {
                    format!("✅ {}/{} ({})", model.provider, model.model, model.context_window)
                } else {
                    format!("{}/{} ({})", model.provider, model.model, model.context_window)
                };

                keyboard_rows.push(vec![InlineKeyboardButton::new(
                    &label,
                    teloxide::types::InlineKeyboardButtonKind::CallbackData(
                        format!("model_{}", i),
                    ),
                )]);
            }

            // Store models in state for callback handling
            state.set_model_list(msg.chat.id, msg.thread_id.clone(), models).await;

            let keyboard = InlineKeyboardMarkup::new(keyboard_rows);

            bot.send_message(msg.chat.id, text)
                .parse_mode(teloxide::types::ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
        }
        _ => {
            // Fallback: just show current model
            let text = match &info.model {
                Some(model) => format!(
                    "🤖 Current model: <code>{model}</code>\n\n\
                     Use <code>/model &lt;name&gt;</code> to switch.",
                ),
                None => "🤖 No model set (using default).\n\n\
                         Use <code>/model &lt;name&gt;</code> to set one."
                    .to_string(),
            };

            bot.send_message(msg.chat.id, text)
                .parse_mode(teloxide::types::ParseMode::Html)
                .await?;
        }
    }

    Ok(())
}

/// Handle model selection callback queries.
pub async fn handle_model_callback(
    bot: Bot,
    query: teloxide::types::CallbackQuery,
    state: HandlerState,
) -> ResponseResult<()> {
    let data = match &query.data {
        Some(d) => d.clone(),
        None => return Ok(()),
    };

    // Parse "model_<index>"
    let index: usize = match data.strip_prefix("model_").and_then(|s| s.parse().ok()) {
        Some(i) => i,
        None => return Ok(()),
    };

    let message = match query.regular_message() {
        Some(m) => m.clone(),
        None => return Ok(()),
    };

    let chat_id = message.chat.id;
    let thread_id = message.thread_id.clone();

    // Get the model list from state
    let models = match state.get_model_list(chat_id, thread_id.clone()).await {
        Some(m) => m,
        None => {
            bot.answer_callback_query(&query.id)
                .text("❌ Model list expired. Use /model again.")
                .await?;
            return Ok(());
        }
    };

    let model = match models.get(index) {
        Some(m) => m.clone(),
        None => {
            bot.answer_callback_query(&query.id)
                .text("❌ Invalid model selection.")
                .await?;
            return Ok(());
        }
    };

    let key = crate::bot::state::chat_key(chat_id.0, thread_id);
    let ctx = crate::bot::state::chat_key_to_context(&key);

    let session = match state.sessions.get_or_create(&ctx).await {
        Ok(s) => s,
        Err(e) => {
            bot.answer_callback_query(&query.id)
                .text(format!("❌ Session error: {e}"))
                .await?;
            return Ok(());
        }
    };

    // Set the model
    let model_id = format!("{}/{}", model.provider, model.model);
    match session.set_model(&model_id).await {
        Ok(()) => {
            bot.answer_callback_query(&query.id)
                .text(format!("✅ Model switched to {model_id}"))
                .await?;

            // Edit the original message to show the new selection
            let _ = bot
                .edit_message_text(chat_id, message.id, format!("🤖 <b>Model switched to:</b> <code>{model_id}</code>"))
                .parse_mode(teloxide::types::ParseMode::Html)
                .await;
        }
        Err(e) => {
            bot.answer_callback_query(&query.id)
                .text(format!("❌ Failed to switch model: {e}"))
                .await?;
        }
    }

    Ok(())
}
