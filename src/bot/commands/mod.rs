pub mod context;
pub mod model;
pub mod sessions;
pub mod tree;

use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

use crate::bot::handler::HandlerState;

/// All TelePi bot commands.
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// Welcome & session info
    #[command(description = "Welcome & session info")]
    Start,

    /// Show usage guide
    #[command(description = "Show usage guide")]
    Help,

    /// Create a new session
    #[command(description = "Create a new session")]
    New,

    /// List and switch sessions
    #[command(description = "List and switch sessions")]
    Sessions,

    /// Resume session in terminal
    #[command(description = "Resume session in terminal")]
    Handback,

    /// Cancel running operation
    #[command(description = "Cancel running operation")]
    Abort,

    /// Re-send last prompt
    #[command(description = "Re-send last prompt")]
    Retry,

    /// Switch AI model
    #[command(description = "Switch AI model")]
    Model,

    /// View conversation tree
    #[command(description = "View conversation tree")]
    Tree,

    /// Show context window usage
    #[command(description = "Show context window usage")]
    Context,
}

/// Register bot commands with Telegram for the command menu.
pub async fn register_menu(bot: &Bot) -> Result<(), teloxide::RequestError> {
    bot.set_my_commands(Command::bot_commands()).await?;
    Ok(())
}

/// Dispatch a command to the appropriate handler.
pub async fn dispatch(bot: Bot, msg: Message, cmd: Command, state: HandlerState) -> ResponseResult<()> {
    match cmd {
        Command::Start | Command::Help => {
            let user = msg.from.as_ref().map(|u| u.id.0).unwrap_or(0);
            if !state.config.is_allowed_user(user) {
                bot.send_message(msg.chat.id, "⛔ You are not authorized.").await?;
                return Ok(());
            }
            send_welcome(&bot, &msg).await?;
        }
        Command::New => sessions::cmd_new(bot, msg, state).await?,
        Command::Sessions => sessions::cmd_sessions(bot, msg, state).await?,
        Command::Handback => sessions::cmd_handback(bot, msg, state).await?,
        Command::Abort => {
            crate::bot::handler::abort_handler(bot, msg, state).await?;
        }
        Command::Retry => {
            crate::bot::handler::retry_handler(bot, msg, state).await?;
        }
        Command::Model => model::cmd_model(bot, msg, state).await?,
        Command::Tree => tree::cmd_tree(bot, msg, state).await?,
        Command::Context => context::cmd_context(bot, msg, state).await?,
    }
    Ok(())
}

/// Send the welcome / help message.
async fn send_welcome(bot: &Bot, msg: &Message) -> ResponseResult<()> {
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
