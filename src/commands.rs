use crate::{Context, Result_, history::TrackUserData};
use serenity::all::{
    Attachment, ComponentInteractionCollector, ComponentInteractionDataKind, CreateEmbed,
    CreateInteractionResponse, CreateMessage, CreateSelectMenu, CreateSelectMenuKind,
    CreateSelectMenuOption,
};
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

/// Search for `query` on `YouTube` and return a list of search results.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn search(ctx: Context<'_>, query: String) -> Result_<()> {
    let guild_id = ctx.guild_id().expect("Should be in a server.");
    let (songbird_manager, has_handler) = super::utils::in_voice(ctx).await?;
    if !has_handler {
        super::utils::join_voice(ctx, None).await?;
    }

    let call = songbird_manager
        .get(guild_id)
        .expect("Should be in a voice channel.");

    let user_data = ctx.data();
    let client = user_data.client.clone();

    // Run the search.
    let mut youtube_search = YoutubeDl::new_search(client.clone(), query);
    let search_results = youtube_search.search(Some(10)).await?.collect::<Vec<_>>();

    // Create a message for the user to pick the result.
    // TODO: make this nice and make it used thumbnails.
    let selection_context = CreateMessage::new()
        .embed(CreateEmbed::new().title("Serach results:"))
        .select_menu(CreateSelectMenu::new(
            "search-select-menu",
            CreateSelectMenuKind::String {
                options: search_results
                    .iter()
                    .map(|meta| {
                        let label = meta.title.clone().unwrap_or("Unknown track".into());
                        let value = meta.source_url.clone().unwrap_or_default();
                        CreateSelectMenuOption::new(label, value)
                    })
                    .collect(),
            },
        ));
    let select_message = ctx
        .channel_id()
        .send_message(ctx.http(), selection_context)
        .await?;

    let mut url = None;

    // Get a single result.
    while let Some(interaction) = ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(120))
        .filter(move |interaction| interaction.data.custom_id == "search-select-menu")
        .await
    {
        url = match interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { ref values } => {
                values.iter().next().cloned()
            }
            _ => None,
        };
        interaction
            .create_response(ctx.http(), CreateInteractionResponse::Acknowledge)
            .await?;

        // After getting the correct selection, remove the message.
        if url.is_some() {
            select_message.delete(ctx.http()).await?;
            break;
        }
    }

    let url = url.unwrap();

    let source = YoutubeDl::new(client, url);

    {
        let q = user_data
            .qs
            .lock()
            .get(&guild_id)
            .expect("Should have been initialized.")
            .clone();
        let mut driver = call.lock().await;

        q.add_from_youtube(source.into(), &mut driver).await?;
    }

    Ok(())
}

/// Try to play a song from the provided URL.
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
    let guild_id = ctx.guild_id().expect("Should be in a server.");
    let (songbird_manager, has_handler) = super::utils::in_voice(ctx).await?;
    if !has_handler {
        super::utils::join_voice(ctx, None).await?;
    }

    let call = songbird_manager
        .get(guild_id)
        .expect("Should be connected to voice.");

    {
        let mut driver = call.lock().await;
        let q = ctx
            .data()
            .qs
            .lock()
            .get(&guild_id)
            .expect("Should have been created.")
            .clone();

        let _ = q.add_from_attachment(file, &mut driver).await?;
    }

    Ok(())
}

/// Subcommands for manipulating the queue.
#[poise::command(
    prefix_command,
    category = "Music",
    subcommands("show", "history", "shuffle"),
    subcommand_required,
    guild_only
)]
pub async fn queue(_: Context<'_>) -> Result_<()> {
    Ok(())
}

/// Show the contents of the queue.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn show(ctx: Context<'_>) -> Result_<()> {
    let guild_id = ctx.guild_id().expect("Should be in a server.");
    let (_, has_handler) = super::utils::in_voice(ctx).await?;
    if !has_handler {
        ctx.reply("Not in a voice channel, good sir!").await?;
        return Ok(());
    }
    let queued = ctx
        .data()
        .qs
        .lock()
        .get(&guild_id)
        .expect("Should have been initialized")
        .clone()
        .current_queue();

    let pages = queued
        .chunks(10)
        .enumerate()
        .map(|(chunk, handles)| {
            let mut page = String::new();
            for (i, handle) in handles.iter().enumerate() {
                match handle.data::<TrackUserData>().as_ref() {
                    TrackUserData::Youtube { title, url: _ } => page.push_str(&format!(
                        "{}. {} (from YouTube)\n",
                        chunk * 10 + i + 1,
                        title
                    )),
                    TrackUserData::Attachment {
                        title,
                        attachment_url: _,
                    } => page.push_str(&format!(
                        "{}. {} (from file attachment)\n",
                        chunk * 10 + i + 1,
                        title
                    )),
                    TrackUserData::HttpStream { url } => {
                        page.push_str(&format!("{}. {} (http stream)\n", chunk * 10 + i + 1, url))
                    }
                }
            }
            page
        })
        .collect::<Vec<_>>();

    super::utils::paginate(ctx, pages).await?;

    Ok(())
}

/// Show the song history.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn history(ctx: Context<'_>) -> Result_<()> {
    let guild_id = ctx.guild_id().expect("Should be in a server.");
    let (_, has_handler) = super::utils::in_voice(ctx).await?;
    if !has_handler {
        ctx.reply("Not in a voice channel, good sir!").await?;
        return Ok(());
    }
    let queued = ctx
        .data()
        .qs
        .lock()
        .get(&guild_id)
        .expect("Should have been initialized")
        .clone()
        .history();

    let pages = queued
        .chunks(10)
        .enumerate()
        .map(|(chunk, handles)| {
            let mut page = String::new();
            for (i, handle) in handles.iter().enumerate() {
                match handle {
                    TrackUserData::Youtube { title, url: _ } => page.push_str(&format!(
                        "{}. {} (from YouTube)\n",
                        chunk * 10 + i + 1,
                        title
                    )),
                    TrackUserData::Attachment {
                        title,
                        attachment_url: _,
                    } => page.push_str(&format!(
                        "{}. {} (from file attachment)\n",
                        chunk * 10 + i + 1,
                        title
                    )),
                    TrackUserData::HttpStream { url } => {
                        page.push_str(&format!("{}. {} (http stream)\n", chunk * 10 + i + 1, url))
                    }
                }
            }
            page
        })
        .collect::<Vec<_>>();

    super::utils::paginate(ctx, pages).await?;

    Ok(())
}

/// Shuffle the queue.
#[poise::command(prefix_command, category = "Music", guild_only)]
pub async fn shuffle(ctx: Context<'_>) -> Result_<()> {
    let guild_id = ctx.guild_id().expect("Should be in a server.");
    let (_, has_handler) = super::utils::in_voice(ctx).await?;
    if !has_handler {
        ctx.reply("Not in a voice channel, good sir!").await?;
        return Ok(());
    }

    ctx.data()
        .qs
        .lock()
        .get(&guild_id)
        .expect("Should be initialized.")
        .clone()
        .shuffle();

    Ok(())
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
