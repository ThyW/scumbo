#![allow(unused)]

use crate::{Context, Result_};
use serenity::all::Attachment;
use songbird::input::{HttpRequest, YoutubeDl};

/// Simple `echo` command for parroting everything the user types.
#[poise::command(prefix_command, category = "Testing")]
pub async fn echo(ctx: Context<'_>, value: Option<String>) -> Result_<()> {
    if let Some(v) = value {
        ctx.say(v).await?;
    } else {
        ctx.say("🤫").await?;
    }
    Ok(())
}

/// Find out the Discord user id of a mentioned user.
#[poise::command(prefix_command, owners_only, category = "Testing")]
pub async fn id(ctx: Context<'_>, user: poise::serenity_prelude::User) -> Result_<()> {
    ctx.say(format!("User `{}` id is: `{}`", user.name, user.id))
        .await?;

    Ok(())
}

/// Show a help message.
#[poise::command(prefix_command, track_edits)]
pub async fn help(ctx: Context<'_>, command: Option<String>) -> Result_<()> {
    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: "\
Type '<prefix>help <command>' for more info on a command.
You can edit your message to the bot and the bot will edit its response.",
        ephemeral: true,
        show_subcommands: true,
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}

/// Join the voice channel that the user is in, alternatively the user can supply a mention of the
/// voice channel for the bot to join.
#[poise::command(prefix_command, guild_only, category = "Music")]
pub async fn join(
    ctx: Context<'_>,
    voice_channel: Option<serenity::model::channel::GuildChannel>,
) -> Result_<()> {
    super::utils::join_voice(ctx, voice_channel).await
}

/// Leave the voice channel, if inside of one.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn leave(ctx: Context<'_>) -> Result_<()> {
    let (songbird_manager, has_handler) = super::utils::in_voice(ctx).await?;

    // Leave the voice channel and notify the user.
    if has_handler {
        songbird_manager.remove(ctx.guild_id().unwrap()).await?;

        ctx.reply("I have left the voice channel.").await?;
    } else {
        ctx.reply("I'm not in a voice channel, you dummy!").await?;
    }
    Ok(())
}

/// Resume playing a song or try to play the first result of the query search.
#[poise::command(
    prefix_command,
    category = "Music",
    guild_only,
    subcommands("search", "url", "file")
)]
pub async fn play(ctx: Context<'_>, query: Option<String>) -> Result_<()> {
    let guild_id = ctx.guild_id().expect("Should be in server.");
    let (songbird_manager, has_handler) = super::utils::in_voice(ctx).await?;
    if !has_handler {
        super::utils::join_voice(ctx, None).await?;
    }

    let call = songbird_manager
        .get(ctx.guild_id().expect("Only in guilds"))
        .expect("Should be connected.");

    match query {
        Some(query_) => {
            let mut driver = call.lock().await;
            let user_data = ctx.data();
            let client = user_data.client.clone();
            let q = user_data
                .qs
                .lock()
                .get(&guild_id)
                .expect("Should have been created when joining.")
                .clone();
            let search = YoutubeDl::new_search(client, query_);

            let _ = q.add_from_youtube(search.into(), &mut driver).await?;
        }
        None => {
            ctx.data()
                .qs
                .lock()
                .get(&guild_id)
                .expect("Should have been created when joining.")
                .resume()?;
        }
    }

    Ok(())
}

/// Search for a query and return a list of search results.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn search(ctx: Context<'_>, query: String) -> Result_<()> {
    todo!()
}

/// Try to play a song from the provided url.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn url(ctx: Context<'_>, url: String) -> Result_<()> {
    let guild_id = ctx.guild_id().expect("Should be in server.");
    let (songbird_manager, has_handler) = super::utils::in_voice(ctx).await?;
    if !has_handler {
        super::utils::join_voice(ctx, None).await?;
    }

    let call = songbird_manager
        .get(ctx.guild_id().expect("Only in guilds"))
        .expect("Should be connected.");

    let mut driver = call.lock().await;
    let user_data = ctx.data();
    let client = user_data.client.clone();
    let q = user_data
        .qs
        .lock()
        .get(&guild_id)
        .expect("Should have been created when joining.")
        .clone();
    let search = HttpRequest::new(client, url.clone());

    let _ = q.add_from_stream(search.into(), url, &mut driver).await?;

    Ok(())
}

/// Play a file attached to the message.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn file(ctx: Context<'_>, file: Attachment) -> Result_<()> {
    todo!()
}

/// Subcommands for maniuplating the queue.
#[poise::command(
    prefix_command,
    category = "Music",
    subcommands("show", "history", "shuffle"),
    subcommand_required,
    guild_only
)]
pub async fn queue(ctx: Context<'_>) -> Result_<()> {
    todo!()
}

/// Show the contents of the queue.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn show(ctx: Context<'_>) -> Result_<()> {
    todo!()
}

/// Show the song history.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn history(ctx: Context<'_>) -> Result_<()> {
    todo!()
}

/// Shuffle the queue.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn shuffle(ctx: Context<'_>) -> Result_<()> {
    todo!()
}

/// Pause the currently playing track.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn pause(ctx: Context<'_>) -> Result_<()> {
    let guild_id = ctx.guild_id().expect("Is in a guild.");
    let (_, has_handler) = super::utils::in_voice(ctx).await?;
    if !has_handler {
        ctx.reply("Not in a voice channel, good sir!").await?;
        return Ok(());
    }

    ctx.data()
        .qs
        .lock()
        .get(&guild_id)
        .expect("Should have been created.")
        .clone()
        .pause()?;

    Ok(())
}

/// Stop all queued tracks.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn stop(ctx: Context<'_>) -> Result_<()> {
    let guild_id = ctx.guild_id().expect("Should be in server.");
    let (_, has_handler) = super::utils::in_voice(ctx).await?;

    if !has_handler {
        ctx.reply("Not in a voice channel, good sir!").await?;
        return Ok(());
    }

    ctx.data()
        .qs
        .lock()
        .get(&guild_id)
        .expect("Should have been created.")
        .stop();

    Ok(())
}
