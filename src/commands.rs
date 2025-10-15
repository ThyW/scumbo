use crate::{Context, Result_};

/// Simple `echo` command for parroting everything the user types.
#[poise::command(prefix_command)]
pub async fn echo(ctx: Context<'_>, value: Option<String>) -> Result_<()> {
    if let Some(v) = value {
        ctx.say(v).await?;
    } else {
        ctx.say("🤫").await?;
    }
    Ok(())
}

/// Find out the Discord user id of a mentioned user.
#[poise::command(prefix_command, owners_only)]
pub async fn id(ctx: Context<'_>, user: poise::serenity_prelude::User) -> Result_<()> {
    ctx.say(format!("User `{}` id is: `{}`", user.name, user.id))
        .await?;

    Ok(())
}

/// Show a help message.
#[poise::command(prefix_command, track_edits)]
pub async fn help(ctx: Context<'_>, command: Option<String>) -> Result_<()> {
    // TODO: configure the help message further.
    let config = poise::builtins::HelpConfiguration {
        extra_text_at_bottom: "\
Type '<prefix>help <command>' for more info on a command.
You can edit your message to the bot and the bot will edit its response.",
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), config).await?;
    Ok(())
}
