use std::collections::VecDeque;

/// A fixed sized buffer for holding up to `capacity` data about tracks played in a single server.
///
/// The default `capacity` is **50**;
#[derive(Clone, Debug)]
pub struct History {
    tracks: VecDeque<TrackUserData>,
    capacity: usize,
}

impl Default for History {
    fn default() -> Self {
        Self {
            tracks: VecDeque::with_capacity(50),
            capacity: 50,
        }
    }
}

impl History {
    /// Creates a new track history.
    pub fn new(capacity: usize) -> Self {
        Self {
            tracks: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    /// Add info about a track into the track data.
    pub fn add(&mut self, data: TrackUserData) {
        // TODO: Think about how duplicate `TrackUserData` should be handled.
        if self.tracks.len() == self.capacity {
            let _ = self.tracks.pop_front();
        }

        self.tracks.push_back(data)
    }

    /// Get the latest song added to history.
    pub fn remove(&mut self) -> Option<TrackUserData> {
        self.tracks.pop_front()
    }

    /// Get the `n-th` previous `TrackUserData`.
    pub fn nth(&mut self, n: usize) -> Option<TrackUserData> {
        self.tracks.remove(n)
    }

    /// Get the `n-th` previous `TrackUserData`, without removing it from the history.
    pub fn peek(&self, n: usize) -> Option<&TrackUserData> {
        self.tracks.get(n)
    }

    /// Return a vector of cloned tracks in the history.
    pub fn list(&self) -> Vec<TrackUserData> {
        self.tracks.iter().cloned().collect()
    }
}

/// This is used to track the `Source` of a `Track` played by the bot.
#[derive(Clone, Debug)]
pub enum TrackUserData {
    Youtube {
        title: String,
        url: String,
    },
    Attachment {
        title: String,
        attachment_url: String,
    },
    HttpStream {
        url: String,
    },
}
