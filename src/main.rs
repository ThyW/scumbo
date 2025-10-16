#![allow(clippy::map_entry)]

use poise::{Framework, FrameworkOptions, PrefixFrameworkOptions};
use serenity::model::gateway::GatewayIntents;

use songbird::{Config, SerenityInit};

mod callbacks;
mod commands;
mod handlers;
mod smq;
mod utils;

// For owner only commands.
pub const OWNER_ID: u64 = 272795263414829057;

// Useful aliases.
pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result_<T> = Result<T, Error>;
pub type Context<'a> = poise::Context<'a, State, Error>;

// Bot state goes here.
pub struct State {}

#[tokio::main]
async fn main() -> Result_<()> {
    // TODO: Possibly setup logging first.

    // Parse `.env` file.
    dotenv::dotenv().expect("cannot load env");

    let token = dotenv::var("DISCORD_TOKEN")?;
    let prefix = dotenv::var("BOT_PREFIX").unwrap_or("!".into());

    // Set all unprivileged intents.
    //
    // Because we want to use prefixes, the `MESSAGE_CONTENT` intent is also necessary.
    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    // Here we setup the command parsing framework. (poise)
    let framework = Framework::builder()
        // Configure the framework.
        .options(FrameworkOptions {
            commands: vec![
                crate::commands::echo(),
                crate::commands::id(),
                crate::commands::help(),
                crate::commands::join(),
                crate::commands::leave(),
            ],
            on_error: crate::callbacks::on_error,
            owners: std::collections::HashSet::from([OWNER_ID.into()]),
            prefix_options: PrefixFrameworkOptions {
                prefix: Some(prefix),
                ..Default::default()
            },
            ..Default::default()
        })
        // Run the framework setup, initializing user data.
        .setup(move |_ctx, _ready, _framework| Box::pin(async move { Ok(State {}) }))
        .build();

    // Create a `songbird` configuration.
    let songbird_config = Config::default()
        .preallocated_tracks(16)
        .use_softclip(false)
        .driver_timeout(Some(std::time::Duration::from_secs(30)));

    // Setup the discord client.
    let mut client = serenity::Client::builder(token, intents)
        .framework(framework)
        .register_songbird_from_config(songbird_config)
        .await
        .expect("client should have been correctly created");

    // Run the bot.
    client.start().await?;

    Ok(())
}
