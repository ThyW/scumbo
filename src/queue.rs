#![allow(unused)]
use crate::{
    Result_,
    history::{History, TrackUserData},
};
use parking_lot::Mutex;
use serenity::all::Attachment;
use songbird::{
    Call,
    driver::Driver,
    events::{Event, EventData, TrackEvent},
    input::{Compose, File, HttpRequest, Input, YoutubeDl},
    tracks::{Track, TrackHandle, TrackResult},
};
use std::{collections::VecDeque, ops::Deref, sync::Arc, time::Duration};

#[derive(Clone, Debug, Default)]
pub struct TrackQueue {
    pub inner: Arc<Mutex<TrackQueueCore>>,
}

#[derive(Debug)]
pub struct Queued(pub TrackHandle);

impl Deref for Queued {
    type Target = TrackHandle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Queued {
    pub fn handle(&self) -> TrackHandle {
        self.0.clone()
    }
}

#[derive(Debug, Default)]
pub struct TrackQueueCore {
    pub queued_tracks: VecDeque<Queued>,
    pub history: History,
}

pub struct QueueHandler {
    pub remote_lock: Arc<Mutex<TrackQueueCore>>,
}

pub struct SongPreloader {
    pub remote_lock: Arc<Mutex<TrackQueueCore>>,
}

impl TrackQueue {
    /// Create a new track queue.
    pub fn new(history_capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TrackQueueCore {
                queued_tracks: VecDeque::new(),
                history: History::new(history_capacity),
            })),
        }
    }

    /// Try to add a track supplied to the bot as an attachment.
    pub async fn add_from_attachment(
        &self,
        attachment: Attachment,
        driver: &mut Driver,
    ) -> Result_<TrackHandle> {
        let data = attachment.download().await?;
        let user_data = TrackUserData::Attachment {
            title: attachment.filename.clone(),
            attachment_url: attachment.url.clone(),
        };
        let track = Track::new_with_data(data.into(), Arc::new(user_data));
        Ok(self.add(track, driver).await)
    }

    /// Add a track from a `YouTube` search.
    pub async fn add_from_youtube(
        &self,
        mut input: Input,
        driver: &mut Driver,
    ) -> Result_<TrackHandle> {
        let metadata = input.aux_metadata().await?;
        let user_data = TrackUserData::Youtube {
            url: metadata.source_url.unwrap_or_default(),
            title: metadata.title.unwrap_or_else(|| "Unknown track".into()),
        };

        let track = Track::new_with_data(input.into(), Arc::new(user_data));
        Ok(self.add(track, driver).await)
    }

    /// Add a track from an HTTP request.
    pub async fn add_from_stream(
        &self,
        mut input: Input,
        url: String,
        driver: &mut Driver,
    ) -> Result_<TrackHandle> {
        let user_data = TrackUserData::HttpStream { url };
        let track = Track::new_with_data(input.into(), Arc::new(user_data));
        Ok(self.add(track, driver).await)
    }

    async fn add(&self, mut track: Track, driver: &mut Driver) -> TrackHandle {
        let preload_time = Self::get_preload_time(&mut track).await;
        self.add_with_preload(track, driver, preload_time)
    }

    async fn get_preload_time(track: &mut Track) -> Option<Duration> {
        let meta = match track.input {
            Input::Lazy(ref mut rec) | Input::Live(_, Some(ref mut rec)) => {
                rec.aux_metadata().await.ok()
            }
            Input::Live(_, None) => None,
        };

        meta.and_then(|meta| meta.duration)
            .map(|d| d.saturating_sub(Duration::from_secs(5)))
    }

    #[inline]
    fn add_with_preload(
        &self,
        mut track: Track,
        driver: &mut Driver,
        preload_time: Option<Duration>,
    ) -> TrackHandle {
        let remote_lock = self.inner.clone();
        track.events.add_event(
            EventData::new(Event::Track(TrackEvent::End), QueueHandler { remote_lock }),
            Duration::ZERO,
        );

        if let Some(time) = preload_time {
            let remote_lock = self.inner.clone();
            track.events.add_event(
                EventData::new(Event::Delayed(time), SongPreloader { remote_lock }),
                Duration::ZERO,
            );
        }

        let (should_play, handle) = {
            let mut inner = self.inner.lock();

            let handle = driver.play(track.pause());
            inner.queued_tracks.push_back(Queued(handle.clone()));

            (inner.queued_tracks.len() == 1, handle)
        };

        if should_play {
            drop(handle.play());
        }

        handle
    }

    /// Get the currently playing track.
    pub fn current(&self) -> Option<TrackHandle> {
        let inner = self.inner.lock();

        inner.queued_tracks.front().map(Queued::handle)
    }

    /// Remove track at `index` without adding it to `History`.
    pub fn dequeue(&self, index: usize) -> Option<Queued> {
        self.modify_queue(|vq| vq.remove(index))
    }

    /// Get the length of the queue.
    pub fn len(&self) -> usize {
        let inner = self.inner.lock();

        inner.queued_tracks.len()
    }

    /// Is the queue empty?
    pub fn is_empty(&self) -> bool {
        let inner = self.inner.lock();

        inner.queued_tracks.is_empty()
    }

    /// Run a `func` to modify the queue.
    pub fn modify_queue<F, O>(&self, func: F) -> O
    where
        F: FnOnce(&mut VecDeque<Queued>) -> O,
    {
        let mut inner = self.inner.lock();
        func(&mut inner.queued_tracks)
    }

    /// Pause the track. It can be resumed later.
    pub fn pause(&self) -> TrackResult<()> {
        let inner = self.inner.lock();

        if let Some(handle) = inner.queued_tracks.front() {
            handle.pause()
        } else {
            Ok(())
        }
    }

    /// Resume a paused track.
    pub fn resume(&self) -> TrackResult<()> {
        let inner = self.inner.lock();

        if let Some(handle) = inner.queued_tracks.front() {
            handle.play()
        } else {
            Ok(())
        }
    }

    /// Stop the current track and remove all further tracks from the queue.
    ///
    /// This does not save the tracks which were not played in the history.
    pub fn clear(&self) -> TrackResult<()> {
        let mut inner = self.inner.lock();

        inner
            .stop_current()
            .and_then(|_| Ok(inner.queued_tracks.clear()))
    }

    /// Stop the queue.
    ///
    /// This operation clears the queue. The tracks stored in the queue will be stored in the
    /// history.
    pub fn stop(&self) {
        let mut inner = self.inner.lock();

        for track in inner.queued_tracks.drain(..) {
            drop(track.stop());
        }
    }

    /// Try to skip up to `n` tracks.
    pub fn skip(&self, mut n: usize) -> TrackResult<()> {
        let inner = self.inner.lock();

        while n > 0 && !inner.queued_tracks.is_empty() {
            inner.stop_current()?;
        }
        Ok(())
    }

    /// Get the contents of the current queue.
    pub fn current_queue(&self) -> Vec<TrackHandle> {
        let inner = self.inner.lock();

        inner.queued_tracks.iter().map(Queued::handle).collect()
    }

    /// Get the track history.
    pub fn history(&self) -> Vec<TrackUserData> {
        let inner = self.inner.lock();

        inner.history.list()
    }

    /// Get the metadata of a previously played track.
    pub fn previous(&self, n: usize) -> Option<TrackUserData> {
        let inner = self.inner.lock();

        inner.history.peek(n).cloned()
    }
}

impl TrackQueueCore {
    fn stop_current(&self) -> TrackResult<()> {
        if let Some(handle) = self.queued_tracks.front() {
            handle.stop()
        } else {
            Ok(())
        }
    }
}
