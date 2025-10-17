use parking_lot::Mutex;
use songbird::{input::AuxMetadata, tracks::TrackHandle};
use std::{collections::VecDeque, sync::Arc};

pub(crate) type QueueHandle = Arc<Mutex<CoreQueue>>;

#[derive(Default)]
pub(crate) struct CoreQueue {
    q: VecDeque<TrackHandle>,
    mq: VecDeque<AuxMetadata>,
    position: usize,
}

#[derive(Default)]
pub struct TrackQueue {
    inner: QueueHandle,
}

pub(crate) struct PreloadHandler {
    inner: QueueHandle,
}
pub(crate) struct QueueHandler {
    inner: QueueHandle,
}
