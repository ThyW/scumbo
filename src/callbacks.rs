use crate::{Error, State};

use poise::{BoxFuture, FrameworkError, serenity_prelude::CacheHttp};

/// Code which executes when a command parsing framework error occurs.
pub fn on_error(err: FrameworkError<'_, State, Error>) -> BoxFuture<'_, ()> {
    Box::pin(async move {
        match err {
            FrameworkError::Setup { error, .. } => println!("setup error: {error}"),
            FrameworkError::EventHandler { error, .. } => {
                println!("framework error: {error}")
            }
            FrameworkError::Command { error, ctx, .. } => {
                println!("command error: {error}");
                let _ = ctx
                    .reply(format!(
                        "The following error occurred when handling a command: {error}"
                    ))
                    .await;
            }
            FrameworkError::SubcommandRequired { ctx } => {
                println!("subcommand required error");
                let _ = ctx.reply("No subcommand provided.").await;
            }
            FrameworkError::CommandPanic { payload, .. } => {
                println!(
                    "Command pannicked with the following payload: {}",
                    payload.unwrap_or("None".into())
                )
            }
            FrameworkError::ArgumentParse {
                error, input, ctx, ..
            } => {
                let input = input.unwrap_or("<No input>".into());
                println!("argument parse error on input: '{}': {error}", &input);
                let _ = ctx
                    .reply(format!(
                        "Failed to parse the command argument: {error}\n\t\ton input: '{}'",
                        input
                    ))
                    .await;
            }
            FrameworkError::CommandStructureMismatch {
                description, ctx, ..
            } => {
                println!("command structure mismatch: {description}");
                let _ = ctx
                    .reply(format!("Mismatched command structure: {description}"))
                    .await;
            }
            FrameworkError::CooldownHit {
                remaining_cooldown,
                ctx,
                ..
            } => {
                println!(
                    "cooldown hints, time remaining {}",
                    remaining_cooldown.as_secs()
                );
                let _ = ctx
                    .reply(format!(
                        "Command is on cooldown, {} seconds remaining!",
                        remaining_cooldown.as_secs()
                    ))
                    .await;
            }
            FrameworkError::MissingBotPermissions {
                missing_permissions,
                ctx,
                ..
            } => {
                println!("missing bot permissions: {}", missing_permissions);
                let _ = ctx
                    .reply(format!(
                        "Bot is missing the following permissions: {}",
                        missing_permissions
                    ))
                    .await;
            }
            FrameworkError::MissingUserPermissions {
                missing_permissions,
                ctx,
                ..
            } => {
                println!(
                    "missing user permissions: {}",
                    missing_permissions.unwrap_or_default()
                );
                let _ = ctx
                    .reply(format!(
                        "User is missing the following permissions: {}",
                        missing_permissions.unwrap_or_default()
                    ))
                    .await;
            }
            FrameworkError::NotAnOwner { ctx, .. } => {
                println!("non-owner tried to invoke an owner command");
                let _ = ctx
                    .reply("Hey, you can't do that, you are not an owner!")
                    .await;
            }
            FrameworkError::GuildOnly { ctx, .. } => {
                let _ = ctx.reply("Guild only command, sorry!").await;
            }
            FrameworkError::DmOnly { ctx, .. } => {
                let _ = ctx.reply("DM only command, sorry!").await;
            }
            FrameworkError::NsfwOnly { ctx, .. } => {
                let _ = ctx.reply("Not in a *freaky* channel ;)").await;
            }
            FrameworkError::CommandCheckFailed { error, ctx, .. } => {
                println!(
                    "command check failed with the following error: {}",
                    error.unwrap_or("<no error>".into())
                );
                let _ = ctx.reply("Command check failed.").await;
            }
            FrameworkError::DynamicPrefix { error, .. } => {
                println!("dynamic prefix function returned an error: {error}");
            }
            FrameworkError::UnknownCommand { ctx, msg, .. } => {
                let _ = msg
                    .reply(ctx.http(), "Not a recognized command, sorry!")
                    .await;
            }
            FrameworkError::UnknownInteraction { .. } => {
                println!("unknown interaction error");
            }
            _ => unreachable!(),
        }
    })
}
