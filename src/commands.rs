use crate::{Context, Result_};

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
