pub mod commands;
pub mod handler;
pub mod keyboard;
pub mod state;
pub mod transport;

use std::sync::Arc;

use teloxide::prelude::*;
use tracing::info;

use crate::config::TelePiConfig;
use crate::pi::registry::SessionRegistry;

use handler::HandlerState;

/// Build and run the Telegram bot.
pub async fn run(config: TelePiConfig) -> anyhow::Result<()> {
    let bot = Bot::new(&config.telegram_bot_token);
    let config = Arc::new(config);

    // Register commands in Telegram menu
    commands::register_menu(&bot).await?;
    info!("registered telegram bot commands");

    let sessions = SessionRegistry::new(config.clone());
    let chat_state = state::BotChatState::new();

    let handler_state = HandlerState {
        config: config.clone(),
        sessions,
        chat_state,
    };

    info!("bot starting polling");

    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<commands::Command>()
                .endpoint(commands::dispatch),
        )
        .branch(
            dptree::filter(|msg: Message| msg.voice().is_some() || msg.audio().is_some())
                .endpoint(handler::voice_handler),
        )
        .branch(
            dptree::filter(|msg: Message| msg.photo().is_some() || msg.document().is_some())
                .endpoint(handler::photo_handler),
        )
        .branch(
            dptree::filter(|msg: Message| msg.text().is_some())
                .endpoint(handler::text_handler),
        );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![handler_state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
