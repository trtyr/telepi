pub mod commands;
pub mod handler;
pub mod keyboard;
pub mod prompt_inbox;
pub mod state;
pub mod transport;

use std::collections::HashMap;
use std::sync::Arc;

use teloxide::prelude::*;
use tracing::{info, warn};

use crate::config::TelePiConfig;
use crate::pi::registry::SessionRegistry;

use handler::HandlerState;

/// Maximum retry attempts for 409 Conflict.
const MAX_CONFLICT_RETRIES: u32 = 5;
/// Delay between retry attempts.
const CONFLICT_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(3);

/// Build and run the Telegram bot.
pub async fn run(config: TelePiConfig) -> anyhow::Result<()> {
    // Build shared HTTP client with timeouts and proxy
    let mut http_builder = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(120));
    if let Some(ref proxy) = config.proxy {
        match reqwest::Proxy::all(proxy) {
            Ok(p) => { http_builder = http_builder.proxy(p); }
            Err(e) => {
                warn!(proxy = %proxy, error = %e, "invalid proxy, ignoring");
            }
        }
    }
    let http_client = http_builder.build()
        .unwrap_or_else(|_| reqwest::Client::new());

    // Bot uses the same proxy-aware client
    let bot = Bot::with_client(&config.telegram_bot_token, http_client.clone());
    let config = Arc::new(config);

    // Startup: retry until Telegram is reachable (network/proxy may be temporarily down)
    {
        use std::time::Duration;
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let result = async {
                bot.delete_webhook().send().await?;
                info!("cleared existing webhook");
                commands::register_menu(&bot).await?;
                info!("registered telegram bot commands");
                Ok::<(), anyhow::Error>(())
            }.await;

            match result {
                Ok(()) => break,
                Err(e) => {
                    let delay = Duration::from_secs(4u64.saturating_pow(attempt.min(6)));
                    warn!(
                        error = %e,
                        attempt = attempt,
                        retry_in = %format!("{}s", delay.as_secs()),
                        "startup failed, retrying..."
                    );
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    let sessions = SessionRegistry::new(config.clone());
    let chat_state = state::BotChatState::new();

    let handler_state = HandlerState {
        config: config.clone(),
        sessions,
        chat_state,
        http: http_client,
        model_lists: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
    };

    let message_handler = Update::filter_message()
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

    let callback_handler = Update::filter_callback_query()
        .endpoint(commands::model::handle_model_callback);

    let handler = dptree::entry()
        .branch(message_handler)
        .branch(callback_handler);
    // Start prompt inbox polling if configured
    let _inbox_handle = prompt_inbox::start_prompt_inbox_polling(
        config.clone(),
        handler_state.clone(),
    );
    // Retry loop for 409 Conflict (another bot instance polling)
    let mut attempt = 0;
    loop {
        attempt += 1;
        info!(attempt, "starting bot polling");

        let mut dispatcher = Dispatcher::builder(bot.clone(), handler.clone())
            .dependencies(dptree::deps![handler_state.clone()])
            .enable_ctrlc_handler()
            .build();

        dispatcher.dispatch().await;

        // If we reach here, polling stopped. Check if we should retry.
        if attempt >= MAX_CONFLICT_RETRIES {
            warn!(attempts = attempt, "polling stopped after max retries");
            break;
        }

        info!(
            attempt,
            delay_secs = CONFLICT_RETRY_DELAY.as_secs(),
            "polling stopped, retrying after delay"
        );
        tokio::time::sleep(CONFLICT_RETRY_DELAY).await;
    }

    Ok(())
}
