use teloxide::prelude::*;
use teloxide::types::{ChatAction, ChatId, MessageId, ParseMode};

/// Telegram message length limit.
pub const TELEGRAM_MESSAGE_LIMIT: usize = 4096;

/// Send a text reply to a message, splitting if necessary.
pub async fn send_text(
    bot: &Bot,
    chat_id: ChatId,
    reply_to: Option<MessageId>,
    text: &str,
) -> Result<Message, teloxide::RequestError> {
    let chunks = split_text(text, TELEGRAM_MESSAGE_LIMIT);
    let mut last_msg = None;

    for (i, chunk) in chunks.iter().enumerate() {
        let mut send = bot
            .send_message(chat_id, chunk.clone())
            .parse_mode(ParseMode::Html);

        if i == 0 {
            if let Some(msg_id) = reply_to {
                send = send.reply_parameters(
                    teloxide::types::ReplyParameters::new(msg_id),
                );
            }
        }

        last_msg = Some(send.await?);
    }

    Ok(last_msg.expect("at least one chunk"))
}

/// Edit an existing message's text.
pub async fn edit_text(
    bot: &Bot,
    chat_id: ChatId,
    message_id: MessageId,
    text: &str,
) -> Result<(), teloxide::RequestError> {
    let chunks = split_text(text, TELEGRAM_MESSAGE_LIMIT);
    if let Some(first) = chunks.first() {
        bot.edit_message_text(chat_id, message_id, first.clone())
            .parse_mode(ParseMode::Html)
            .await?;
    }
    Ok(())
}

/// Send a "typing" chat action.
pub async fn send_typing(bot: &Bot, chat_id: ChatId) -> Result<(), teloxide::RequestError> {
    bot.send_chat_action(chat_id, ChatAction::Typing).await?;
    Ok(())
}

/// Split text into chunks that fit within Telegram's message limit.
fn split_text(text: &str, limit: usize) -> Vec<String> {
    if text.len() <= limit {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < text.len() {
        let end = std::cmp::min(start + limit, text.len());
        if end < text.len() {
            if let Some(newline_pos) = text[start..end].rfind('\n') {
                chunks.push(text[start..start + newline_pos + 1].to_string());
                start += newline_pos + 1;
                continue;
            }
        }
        chunks.push(text[start..end].to_string());
        start = end;
    }

    chunks
}
