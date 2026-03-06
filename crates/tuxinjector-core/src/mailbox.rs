use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicU8, Ordering};

const EMPTY: u8 = 255;

// Double-buffered lock-free mailbox. Only the most recent value survives -
// if the producer outpaces the consumer, stale values get silently dropped.
// Used for config snapshots where old data is worthless anyway.
pub struct AtomicMailbox<T> {
    slots: [UnsafeCell<MaybeUninit<T>>; 2],
    ready: AtomicU8, // slot index with data, or EMPTY
}

// SAFETY: the atomic index is our synchronization. The swap protocol ensures
// only one side ever touches a given slot at a time.
unsafe impl<T: Send> Send for AtomicMailbox<T> {}
unsafe impl<T: Send> Sync for AtomicMailbox<T> {}

impl<T> AtomicMailbox<T> {
    pub fn new() -> Self {
        Self {
            slots: [
                UnsafeCell::new(MaybeUninit::uninit()),
                UnsafeCell::new(MaybeUninit::uninit()),
            ],
            ready: AtomicU8::new(EMPTY),
        }
    }

    // Always succeeds - overwrites whatever was sitting there before
    pub fn send(&self, val: T) {
        // Write into whichever slot ISN'T currently marked ready
        let cur = self.ready.load(Ordering::Acquire);
        let dst = if cur == 0 { 1 } else { 0 };

        unsafe {
            let slot = &mut *self.slots[dst as usize].get();
            slot.write(val);
        }

        let prev = self.ready.swap(dst, Ordering::Release);

        // drop the old value if nobody eated it
        if prev != EMPTY && prev != dst {
            unsafe {
                let old_slot = &mut *self.slots[prev as usize].get();
                old_slot.assume_init_drop();
            }
        }
    }

    // Grab the latest value, or none if nothing pending
    pub fn recv(&self) -> Option<T> {
        let idx = self.ready.swap(EMPTY, Ordering::Acquire);
        if idx == EMPTY {
            return None;
        }

        // we atomically claimed this slot, safe to read
        let val = unsafe { (*self.slots[idx as usize].get()).assume_init_read() };
        Some(val)
    }

    pub fn has_data(&self) -> bool {
        self.ready.load(Ordering::Relaxed) != EMPTY
    }
}

impl<T> Default for AtomicMailbox<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for AtomicMailbox<T> {
    fn drop(&mut self) {
        // clean up any unconsumed value
        let idx = *self.ready.get_mut();
        if idx != EMPTY {
            unsafe {
                self.slots[idx as usize].get_mut().assume_init_drop();
            }
        }
    }
}

impl<T> std::fmt::Debug for AtomicMailbox<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pending = self.ready.load(Ordering::Relaxed) != EMPTY;
        f.debug_struct("AtomicMailbox")
            .field("has_data", &pending)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn send_recv_basic() {
        let mb = AtomicMailbox::new();
        assert!(mb.recv().is_none());
        mb.send(42u32);
        assert_eq!(mb.recv(), Some(42));
        assert!(mb.recv().is_none());
    }

    #[test]
    fn newest_wins() {
        let mb = AtomicMailbox::new();
        mb.send(1u32);
        mb.send(2u32);
        mb.send(3u32);
        // should only get the latest
        assert_eq!(mb.recv(), Some(3));
        assert!(mb.recv().is_none());
    }

    #[test]
    fn threaded_newest_wins() {
        let mb = Arc::new(AtomicMailbox::new());
        let mb2 = Arc::clone(&mb);

        let n = 50_000u64;

        let producer = std::thread::spawn(move || {
            for i in 0..n {
                mb2.send(i);
            }
        });

        let mut last = None;
        let mut total = 0u64;

        producer.join().unwrap();

        // drain whatever survived
        loop {
            match mb.recv() {
                Some(v) => {
                    if let Some(prev) = last {
                        assert!(v >= prev, "went backward: {v} < {prev}");
                    }
                    last = Some(v);
                    total += 1;
                }
                None => break,
            }
        }

        // at minimum one value should have made it through
        assert!(total >= 1);
    }

    #[test]
    fn drop_unconsumed() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

        #[derive(Debug)]
        #[allow(dead_code)]
        struct Tracker(u32);
        impl Drop for Tracker {
            fn drop(&mut self) {
                DROP_COUNT.fetch_add(1, Ordering::Relaxed);
            }
        }

        DROP_COUNT.store(0, Ordering::Relaxed);

        let mb = AtomicMailbox::new();
        mb.send(Tracker(1));
        mb.send(Tracker(2)); // Tracker(1) dropped here
        mb.send(Tracker(3)); // Tracker(2) dropped here
        // Tracker(3) still sitting in the mailbox
        drop(mb);

        assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 3);
    }
}
