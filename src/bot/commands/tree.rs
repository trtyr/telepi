use teloxide::prelude::*;

use crate::bot::handler::HandlerState;
use crate::bot::transport;
use crate::pi::tree;

/// /tree — View the conversation tree
pub async fn cmd_tree(bot: Bot, msg: Message, state: HandlerState) -> ResponseResult<()> {
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
    let workspace = &info.workspace;

    // Find session directories for this workspace
    let session_dirs = tree::find_session_dirs(workspace);

    if session_dirs.is_empty() {
        bot.send_message(msg.chat.id, "🌳 No sessions found for this workspace.")
            .await?;
        return Ok(());
    }

    // Find the latest session file
    let mut all_entries = Vec::new();
    for dir in &session_dirs {
        if let Some(jsonl_path) = tree::find_latest_session_file(dir) {
            match tree::parse_session_jsonl(&jsonl_path) {
                Ok(entries) => all_entries.extend(entries),
                Err(_) => continue,
            }
        }
    }

    if all_entries.is_empty() {
        bot.send_message(msg.chat.id, "🌳 No session data found.")
            .await?;
        return Ok(());
    }

    let tree_nodes = tree::build_tree(all_entries);
    let rendered = tree::render_tree(&tree_nodes, 4, 30);

    let text = format!("🌳 <b>Session Tree</b>\n\n<pre>{rendered}</pre>");

    transport::send_text(&bot, msg.chat.id, Some(msg.id), &text).await?;

    Ok(())
}

/// /branch — Navigate to a tree entry
pub async fn cmd_branch(bot: Bot, msg: Message, _state: HandlerState) -> ResponseResult<()> {
    // Parse the entry ID from the command arguments
    let args: Vec<&str> = msg.text().map(|t| t.split_whitespace().skip(1).collect()).unwrap_or_default();

    if args.is_empty() {
        bot.send_message(
            msg.chat.id,
            "🌿 Usage: /branch <entry-id>\n\nUse /tree to see available entries.",
        )
        .await?;
        return Ok(());
    }

    let _entry_id = args[0];

    // TODO: Implement actual branch navigation
    // This would involve setting the Pi session's current position
    bot.send_message(msg.chat.id, "🌿 Branch navigation — coming soon.")
        .await?;

    Ok(())
}

/// /label — Label a tree entry
pub async fn cmd_label(bot: Bot, msg: Message, _state: HandlerState) -> ResponseResult<()> {
    let args: Vec<&str> = msg.text().map(|t| t.split_whitespace().skip(1).collect()).unwrap_or_default();

    if args.is_empty() {
        bot.send_message(
            msg.chat.id,
            "🏷️ Usage: /label <entry-id> <name>\n\nUse /tree to see available entries.",
        )
        .await?;
        return Ok(());
    }

    if args.len() < 2 {
        bot.send_message(msg.chat.id, "🏷️ Usage: /label <entry-id> <name>")
            .await?;
        return Ok(());
    }

    let _entry_id = args[0];
    let _label = args[1..].join(" ");

    // TODO: Implement actual labeling (store in state)
    bot.send_message(msg.chat.id, "🏷️ Tree labeling — coming soon.")
        .await?;

    Ok(())
}
