use crate::{Context, Result_};

/// This is an auxiliary function which does the "joining" to a voice channel. It is here, because
/// both `join` and `play` commands are used to join a voice channel.
pub async fn join_voice(
    ctx: Context<'_>,
    voice_channel: Option<serenity::model::channel::GuildChannel>,
) -> Result_<()> {
    let guild_id = ctx.guild_id().expect("User should be in a guild.");

    // Check if the provided channel is a voice channel.
    if let Some(vc) = voice_channel.as_ref()
        && !matches!(vc.kind, serenity::all::ChannelType::Voice)
    {
        ctx.reply("Are you trying to trick me?! *That* is not a **voice** channel.")
            .await?;
        return Ok(());
    }

    // Use the provided voice channel or get the channel the user is in.
    let channel_id = voice_channel.map(|channel| channel.id).or_else(|| {
        ctx.guild()
            .unwrap()
            .voice_states
            .get(&ctx.author().id)
            .and_then(|voice_state| voice_state.channel_id)
    });

    let connect_to = match channel_id {
        Some(ch) => ch,
        None => {
            ctx.reply("You are not in a voice channel, good sir!")
                .await?;

            return Ok(());
        }
    };

    let songbird_manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Created at initialization.")
        .clone();

    let connection_result = songbird_manager.join(guild_id, connect_to).await;
    match connection_result {
        Ok(driver) => {
            let mut driver = driver.lock().await;
            ctx.data()
                .qs
                .lock()
                .insert(guild_id, super::queue::TrackQueue::new(50));
            // TODO: All `TrackEvent` handlers for the queue should go here.
            driver.add_global_event(
                songbird::TrackEvent::Error.into(),
                crate::handlers::TrackErrorHandler,
            );
        }
        Err(e) => {
            ctx.reply(format!("Could not join the voice channel because: {e}"))
                .await?;
        }
    }

    Ok(())
}

/// Used to check if the bot is in a voice channel.
pub async fn in_voice(ctx: Context<'_>) -> Result_<(std::sync::Arc<songbird::Songbird>, bool)> {
    let guild_id = ctx.guild_id().expect("Should be in a guild.");

    // Get the voice manager.
    let songbird_manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Should be initilaized")
        .clone();

    Ok((
        songbird_manager.clone(),
        songbird_manager.get(guild_id).is_some(),
    ))
}
