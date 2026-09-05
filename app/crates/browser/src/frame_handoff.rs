//! Bounded ownership transfer from CEF's UI thread to gpui. A slow app
//! drops the oldest queued frame, retaining the final frame of an animation.
//! Only owned pixels/textures enter this channel; the CEF source is already
//! copied, and GPU completion has already been awaited, before publication.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

use crate::render::FrameSource;

const CAPACITY: usize = 8;

pub(crate) struct PendingFrame {
    pub browser: i32,
    pub seq: u64,
    pub source: FrameSource,
    pub arrived_us: u64,
}

struct Mailbox<T> {
    tx: async_channel::Sender<T>,
    rx: async_channel::Receiver<T>,
}

impl<T> Mailbox<T> {
    fn new(capacity: usize) -> Self {
        let (tx, rx) = async_channel::bounded(capacity);
        Self { tx, rx }
    }

    /// Never waits for the consumer. If full, atomically replace the oldest
    /// entry rather than lose the newest (possibly final) browser frame.
    fn publish(&self, value: T) -> bool {
        self.tx.force_send(value).expect("mailbox owns its receiver").is_some()
    }

    /// Bound each drain even while the producer continues painting. Returned
    /// values belong entirely to the caller: no channel guard spans a draw.
    fn drain(&self) -> Vec<T> {
        (0..self.rx.capacity().unwrap())
            .map_while(|_| self.rx.try_recv().ok())
            .collect()
    }
}

static MAILBOX: OnceLock<Mailbox<PendingFrame>> = OnceLock::new();
static ASKED: AtomicU64 = AtomicU64::new(0);
static TAKEN: AtomicU64 = AtomicU64::new(0);
static REPLACED: AtomicU64 = AtomicU64::new(0);

pub(crate) fn publish(frame: PendingFrame) {
    ASKED.fetch_add(1, Ordering::Relaxed);
    if MAILBOX.get_or_init(|| Mailbox::new(CAPACITY)).publish(frame) {
        REPLACED.fetch_add(1, Ordering::Relaxed);
    }
    crate::pump::schedule_pump(0);
}

pub(crate) fn drain() -> Vec<PendingFrame> {
    let Some(mailbox) = MAILBOX.get() else { return Vec::new() };
    let frames = mailbox.drain();
    TAKEN.fetch_add(frames.len() as u64, Ordering::Relaxed);
    frames
}

pub(crate) fn counters() -> String {
    format!(
        "handoff_asked={} handoff_taken={} handoff_replaced={} handoff_pending={}",
        ASKED.load(Ordering::Relaxed),
        TAKEN.load(Ordering::Relaxed),
        REPLACED.load(Ordering::Relaxed),
        MAILBOX.get().map_or(0, |mailbox| mailbox.rx.len()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn paused_consumer_keeps_the_newest_frames_in_order() {
        let mailbox = Mailbox::new(3);
        let mut replaced = 0;
        for seq in 1..=100 {
            replaced += usize::from(mailbox.publish(seq));
        }
        assert_eq!(replaced, 97);
        assert_eq!(mailbox.drain(), vec![98, 99, 100]);
        assert!(mailbox.drain().is_empty());
    }

    #[test]
    fn publication_transfers_ownership_without_waiting_for_a_draw() {
        let mailbox = Arc::new(Mailbox::new(1));
        let producer = mailbox.clone();
        let texture = Arc::new(42);
        let weak = Arc::downgrade(&texture);
        producer.publish(texture);
        let drawing = mailbox.drain().pop().unwrap();
        std::thread::spawn(move || {
            for seq in 1..=100 {
                producer.publish(Arc::new(seq));
            }
        }).join().unwrap();
        assert_eq!(*drawing, 42);
        assert!(weak.upgrade().is_some());
        drop(drawing);
        assert!(weak.upgrade().is_none());
        assert_eq!(**mailbox.drain().first().unwrap(), 100);
    }
}
