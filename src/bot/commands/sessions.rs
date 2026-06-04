use teloxide::prelude::*;
use crate::bot::handler::HandlerState;

/// /new — Create a new Pi session for this chat
pub async fn cmd_new(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
    let key = crate::bot::state::chat_key(msg.chat.id.0, msg.thread_id.clone());
    let ctx = crate::bot::state::chat_key_to_context(&key);

    state.sessions.remove(&ctx).await;
    match state.sessions.get_or_create(&ctx).await {
        Ok(session) => {
            let info = session.info();
            bot.send_message(
                msg.chat.id,
                format!("✅ New session created.\nWorkspace: `{}`", info.workspace.display()),
            )
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("❌ Failed to create session: {e}"))
                .await?;
        }
    }

    Ok(())
}

/// /sessions — List all sessions
pub async fn cmd_sessions(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
    let sessions = state.sessions.list().await;

    if sessions.is_empty() {
        bot.send_message(msg.chat.id, "No active sessions.").await?;
        return Ok(());
    }

    let mut text = String::from("<b>Active Sessions:</b>\n\n");
    for (i, info) in sessions.iter().enumerate() {
        text.push_str(&format!(
            "{}. <code>{}</code> — {}\n",
            i + 1,
            info.session_id,
            info.workspace.display(),
        ));
    }

    bot.send_message(msg.chat.id, text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}

/// /handback — Resume session in terminal
pub async fn cmd_handback(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
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

    let text = format!(
        "🔄 To resume this session in your terminal:\n\n\
         <code>PI_SESSION_PATH={} pi</code>\n\n\
         Or use <code>/handoff</code> from the Pi CLI.",
        info.session_path.display(),
    );

    bot.send_message(msg.chat.id, text)
        .parse_mode(teloxide::types::ParseMode::Html)
        .await?;

    Ok(())
}
