use teloxide::prelude::*;
use crate::bot::handler::HandlerState;

/// /model — Show or switch the AI model
pub async fn cmd_model(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
    let key = crate::bot::state::chat_key(msg.chat.id.0, msg.thread_id.clone());
    let ctx = crate::bot::state::chat_key_to_context(&key);

    let session = match state.sessions.get_or_create(&ctx).await {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(msg.chat.id, format!("❌ Session error: {e}")).await?;
            return Ok(());
        }
    };

    let info = session.info();

    let text = match &info.model {
        Some(model) => format!("🤖 Current model: <code>{model}</code>\n\nUse the picker to switch models."),
        None => "🤖 No model set (using default).".to_string(),
    };

    bot.send_message(msg.chat.id, text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}
