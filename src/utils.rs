use poise::CreateReply;
use serenity::all::{CreateEmbed, Http};

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
        ctx.send(reply("Error", "_That_ is not a **voice** channel."))
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
            ctx.send(reply("Error", "You are not in a voice channel, good sir!"))
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
            // This is abhorrent.
            let token = dotenv::var("DISCORD_TOKEN").expect("Should be loaded.");
            let http = Http::new(&token);
            driver.add_global_event(
                songbird::TrackEvent::Play.into(),
                crate::handlers::ResumeHandler((ctx.channel_id(), http)),
            );
            driver.add_global_event(
                songbird::TrackEvent::Error.into(),
                crate::handlers::TrackErrorHandler,
            );

            ctx.send(reply(
                "Info",
                format!(
                    "Joined voice channel: {}",
                    ctx.guild()
                        .expect("Should be in a guild")
                        .channels
                        .get(&connect_to)
                        .expect("Should exist."),
                ),
            ))
            .await?;
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

/// Custom implementation of pagination based on `poise::builtin::paginate`.
pub async fn paginate(ctx: Context<'_>, pages: Vec<String>) -> Result<(), serenity::Error> {
    // Define some unique identifiers for the navigation buttons
    let ctx_id = ctx.id();
    let prev_button_id = format!("{}prev", ctx_id);
    let next_button_id = format!("{}next", ctx_id);

    // Send the embed with the first page as content
    let reply = {
        let components = serenity::all::CreateActionRow::Buttons(vec![
            serenity::all::CreateButton::new(&prev_button_id).emoji('◀'),
            serenity::all::CreateButton::new(&next_button_id).emoji('▶'),
        ]);

        CreateReply::default()
            .embed(serenity::all::CreateEmbed::default().description(&pages[0]))
            .components(vec![components])
    };

    ctx.send(reply).await?;

    // Loop through incoming interactions with the navigation buttons
    let mut current_page = 0;
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
        // button was pressed
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        // Time-out when no navigation button has been pressed for 24 hours.
        .timeout(std::time::Duration::from_secs(3600 * 24))
        .await
    {
        // Depending on which button was pressed, go to next or previous page
        if press.data.custom_id == next_button_id {
            current_page += 1;
            if current_page >= pages.len() {
                current_page = 0;
            }
        } else if press.data.custom_id == prev_button_id {
            current_page = current_page.checked_sub(1).unwrap_or(pages.len() - 1);
        } else {
            // This is an unrelated button interaction
            continue;
        }

        // Update the message with the new page contents
        press
            .create_response(
                ctx.serenity_context(),
                serenity::all::CreateInteractionResponse::UpdateMessage(
                    serenity::all::CreateInteractionResponseMessage::new()
                        .embed(serenity::all::CreateEmbed::new().description(&pages[current_page])),
                ),
            )
            .await?;
    }

    Ok(())
}

/// Used to quickly create a reply embed.
pub fn reply(title: impl Into<String>, content: impl Into<String>) -> CreateReply {
    CreateReply::default()
        .embed(CreateEmbed::new().title(title).description(content))
        .reply(true)
}
