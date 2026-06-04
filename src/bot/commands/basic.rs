use std::sync::Arc;
use teloxide::prelude::*;
use crate::bot::handler::HandlerState;
use crate::config::TelePiConfig;

/// /start — Welcome message
pub async fn cmd_start(bot: Bot, msg: Message, config: Arc<TelePiConfig>) -> ResponseResult<()> {
    let user = msg.from().map(|u| u.id.0).unwrap_or(0);
    if !config.is_allowed_user(user) {
        bot.send_message(msg.chat.id, "⛔ You are not authorized.").await?;
        return Ok(());
    }

    let text = concat!(
        "👋 <b>Welcome to TelePi</b>\n\n",
        "Telegram bridge for the Pi coding agent.\n\n",
        "<b>Commands:</b>\n",
        "/new — Create a new session\n",
        "/sessions — List and switch sessions\n",
        "/handback — Resume session in terminal\n",
        "/model — Switch AI model\n",
        "/tree — View conversation tree\n",
        "/context — Show context window usage\n",
        "/retry — Re-send last prompt\n",
        "/abort — Cancel running operation\n",
        "/help — Show this message\n\n",
        "Send me a text message, voice note, or photo to interact with Pi.",
    );

    bot.send_message(msg.chat.id, text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}

/// /help — Usage guide
pub async fn cmd_help(bot: Bot, msg: Message, config: Arc<TelePiConfig>) -> ResponseResult<()> {
    cmd_start(bot, msg, config).await
}
