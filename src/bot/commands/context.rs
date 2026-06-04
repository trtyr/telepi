use teloxide::prelude::*;
use crate::bot::handler::HandlerState;

/// /context — Show context window usage
pub async fn cmd_context(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
    let key = crate::bot::state::chat_key(msg.chat.id.0, msg.thread_id.clone());
    let ctx = crate::bot::state::chat_key_to_context(&key);

    let session = match state.sessions.get_or_create(&ctx).await {
        Ok(s) => s,
        Err(e) => {
            bot.send_message(msg.chat.id, format!("❌ Session error: {e}")).await?;
            return Ok(());
        }
    };

    let stats = session.stats().await;

    let text = format!(
        "📊 <b>Context Window Usage</b>\n\n\
         Session: <code>{}</code>\n\
         Messages: {}\n\
         Tokens: {} in / {} out",
        stats.session_id, stats.total_messages, stats.tokens_in, stats.tokens_out,
    );

    bot.send_message(msg.chat.id, text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}
