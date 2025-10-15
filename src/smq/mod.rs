#![allow(unused)]
//! # About
//! This file contains the implementation of a shared multi-queue.
//!
//! A shared multi-queue or an `SMQ` is a data structure which consists of multiple queues. Each of
//! these queues, however, holds only a `handle` to an item. The items are shared across all
//! queues, when the last `handle` to an item is removed from the queues, the item as a whole is
//! removed.
//!
//! The purpose of an `SMQ` in this project is to have a data structure which holds one track queue
//! per guild (discord server). An `SMQ` allows to do this in a memory efficient manner, keeping
//! track information and metadata stored a single time, globally.
//!
//! # Operations
//!
//! - `dequeue` - when this operation is executed on a specified queue, an item `handle` is removed and the item is
//!   retrieved, **cloned** and returned. This operation checks whether this was the last `handle`
//!   for a given item. If so, the item is removed from the multi-queue.
//!
//! - `enqueue`- when this operation is executed on a specified queue, the item is saved, a handle for that item is
//!   created and inserted into the specified queue. After, when another queue tries to insert the
//!   same item, a new handle is created and the number of items in the queues is incremented.
use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
    marker::PhantomData,
};

pub trait QId: Eq + Hash + Copy {}
pub trait QItemId: Eq + Hash + Clone {}
pub trait QItem<Id: QItemId>: Clone {}

struct QueueItemHandle<Item: QItem<ItemId>, ItemId: QItemId> {
    item: Item,
    count: usize,
    mar: PhantomData<ItemId>,
}

impl<Item: QItem<ItemId>, ItemId: QItemId> QueueItemHandle<Item, ItemId> {
    fn new(item: Item) -> Self {
        Self {
            item,
            count: 0,
            mar: PhantomData,
        }
    }

    fn enq(&mut self) {
        self.count += 1;
    }

    fn deq(&mut self) -> Item {
        self.count -= 1;
        self.item.clone()
    }

    fn no_refs(&self) -> bool {
        self.count == 0
    }
}

struct Queue<ItemId: QItemId> {
    q: VecDeque<ItemId>,
}

impl<ItemId: QItemId> Queue<ItemId> {
    fn new() -> Self {
        Self { q: VecDeque::new() }
    }

    fn with_capacity(capacity: usize) -> Self {
        Self {
            q: VecDeque::with_capacity(capacity),
        }
    }

    fn enq(&mut self, id: ItemId) {
        self.q.push_front(id);
    }

    fn deq(&mut self) -> Option<ItemId> {
        self.q.pop_back()
    }
}

pub struct SharedMultiQueue<Id: QId, Item: QItem<ItemId>, ItemId: QItemId> {
    items: HashMap<ItemId, QueueItemHandle<Item, ItemId>>,
    qs: HashMap<Id, Queue<ItemId>>,
}

impl<Id: QId, Item: QItem<ItemId>, ItemId: QItemId> SharedMultiQueue<Id, Item, ItemId> {
    pub fn new() -> Self {
        Self {
            items: Default::default(),
            qs: Default::default(),
        }
    }

    pub fn enq(&mut self, qid: Id, item: Item, item_id: ItemId) {
        if !self.qs.contains_key(&qid) {
            let _ = self.qs.insert(qid, Queue::new());
        }

        if !self.items.contains_key(&item_id) {
            let _ = self
                .items
                .insert(item_id.clone(), QueueItemHandle::new(item));
        }
        let mut handle = self.items.get_mut(&item_id).expect("handle created");
        let mut q = self.qs.get_mut(&qid).expect("queue created");

        handle.enq();
        q.enq(item_id);
    }

    pub fn deq(&mut self, qid: Id) -> Option<Item> {
        let q = self.qs.get_mut(&qid)?;
        let item_id = q.deq()?;

        if !self.items.contains_key(&item_id) {
            return None;
        }

        let handle = self.items.get_mut(&item_id)?;
        let item = handle.deq();
        if handle.no_refs() {
            let _ = self.items.remove(&item_id);
            self.items.shrink_to_fit();
        }

        Some(item)
    }
}
