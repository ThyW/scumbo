use serenity::{
    all::{ChannelId, CreateEmbed, CreateMessage, Http},
    async_trait,
};
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler};
use std::sync::Arc;

use crate::{
    history::TrackUserData,
    queue::{QueueHandler, SongPreloader},
};

pub struct TrackErrorHandler;

#[async_trait]
impl VoiceEventHandler for TrackErrorHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                println!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

#[async_trait]
impl VoiceEventHandler for QueueHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let mut inner = self.remote_lock.lock();

        match ctx {
            EventContext::Track(ts) => {
                if inner.queued_tracks.front()?.uuid() != ts.first()?.1.uuid() {
                    return None;
                }
            }
            _ => return None,
        }

        let old = inner.queued_tracks.pop_front();
        if let Some(track) = old {
            inner
                .history
                .add(Arc::unwrap_or_clone(track.data::<TrackUserData>()))
        }

        // Keep going until we find one track which works, or we run out.
        while let Some(new) = inner.queued_tracks.front() {
            if new.play().is_err() {
                // Discard files which cannot be used for whatever reason.
                inner.queued_tracks.pop_front();
            } else {
                break;
            }
        }

        None
    }
}

#[async_trait]
impl VoiceEventHandler for SongPreloader {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let inner = self.remote_lock.lock();

        if let Some(track) = inner.queued_tracks.get(1) {
            // This is the sync-version so that we can fire and ignore
            drop(track.0.make_playable());
        }

        None
    }
}

pub struct ResumeHandler(pub (ChannelId, Http));

#[async_trait]
impl VoiceEventHandler for ResumeHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        match ctx {
            EventContext::Track(track) => {
                if let Some((_, handle)) = track.first() {
                    let title = handle.data::<TrackUserData>().title();
                    let (channel_id, http) = &self.0;
                    let _ = channel_id
                        .send_message(
                            http,
                            CreateMessage::new()
                                .embed(CreateEmbed::new().title("Now playing").description(title)),
                        )
                        .await;
                };
                return None;
            }
            _ => None,
        }
    }
}
