use teloxide::prelude::*;
use teloxide::types::{ChatAction, ChatId, MessageId, ParseMode};

/// Telegram message length limit.
pub const TELEGRAM_MESSAGE_LIMIT: usize = 4096;

/// Maximum retry attempts for network errors.
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff (multiplied by attempt number).
const BASE_DELAY: std::time::Duration = std::time::Duration::from_secs(2);

/// Retry a request up to MAX_RETRIES times with exponential backoff.
async fn with_retry<T, F, Fut>(f: F) -> Result<T, teloxide::RequestError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, teloxide::RequestError>>,
{
    let mut last_err = None;
    for attempt in 0..MAX_RETRIES {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                // Only retry on network errors, not API errors
                if matches!(e, teloxide::RequestError::Network(_)) && attempt < MAX_RETRIES - 1 {
                    let delay = BASE_DELAY * (attempt + 1);
                    tracing::warn!(attempt, delay = ?delay, error = %e, "request failed, retrying...");
                    tokio::time::sleep(delay).await;
                    last_err = Some(e);
                } else {
                    return Err(e);
                }
            }
        }
    }
    Err(last_err.unwrap())
}

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

        let bot = bot.clone();
        let chat_id = chat_id;
        let chunk = chunk.clone();
        let reply_to = if i == 0 { reply_to } else { None };
        last_msg = Some(with_retry(|| {
            let mut s = bot.send_message(chat_id, chunk.clone()).parse_mode(ParseMode::Html);
            if let Some(msg_id) = reply_to {
                s = s.reply_parameters(teloxide::types::ReplyParameters::new(msg_id));
            }
            s.send()
        }).await?);
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
        let bot = bot.clone();
        let text = first.clone();
        with_retry(|| {
            bot.edit_message_text(chat_id, message_id, text.clone())
                .parse_mode(ParseMode::Html)
                .send()
        }).await?;
    }
    Ok(())
}

/// Send a "typing" chat action.
pub async fn send_typing(bot: &Bot, chat_id: ChatId) -> Result<(), teloxide::RequestError> {
    let bot = bot.clone();
    with_retry(|| {
        bot.send_chat_action(chat_id, ChatAction::Typing).send()
    }).await?;
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
