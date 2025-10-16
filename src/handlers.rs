use serenity::async_trait;
use songbird::{Event, EventContext, EventHandler as VoiceEventHandler};

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
