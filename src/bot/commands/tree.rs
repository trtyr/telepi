use teloxide::prelude::*;
use crate::bot::handler::HandlerState;

/// /tree — View the conversation tree
pub async fn cmd_tree(bot: Bot, msg: Message, _state: HandlerState) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, "🌳 Session tree view — not yet implemented.")
        .await?;
    Ok(())
}

/// /branch — Navigate to a tree entry
pub async fn cmd_branch(bot: Bot, msg: Message, _state: HandlerState) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, "🌿 Branch navigation — not yet implemented.")
        .await?;
    Ok(())
}

/// /label — Label a tree entry
pub async fn cmd_label(bot: Bot, msg: Message, _state: HandlerState) -> ResponseResult<()> {
    bot.send_message(msg.chat.id, "🏷️ Tree labeling — not yet implemented.")
        .await?;
    Ok(())
}
