use arc_swap::ArcSwap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// Lock-free read-copy-update cell.
// Single writer, many readers, zero locks. Readers get a cheap Arc guard
// and the writer just atomically swaps the pointer underneath them.
pub struct RcuCell<T> {
    inner: ArcSwap<T>,
    ver: AtomicU64,
}

impl<T> RcuCell<T> {
    pub fn new(val: T) -> Self {
        Self {
            inner: ArcSwap::from_pointee(val),
            ver: AtomicU64::new(0),
        }
    }

    // Swap in a new value. Existing readers keep their guards alive -
    // that's the whole point of RCU.
    pub fn publish(&self, val: T) {
        self.inner.store(Arc::new(val));
        self.ver.fetch_add(1, Ordering::Release);
    }

    // Wait-free snapshot
    pub fn load(&self) -> arc_swap::Guard<Arc<T>> {
        self.inner.load()
    }

    // Monotonically increasing counter, bumped on every publish().
    // Handy for "did anything change since I last checked?"
    pub fn version(&self) -> u64 {
        self.ver.load(Ordering::Acquire)
    }
}

impl<T: Default> Default for RcuCell<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for RcuCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let g = self.inner.load();
        f.debug_struct("RcuCell")
            .field("value", &*g)
            .field("version", &self.version())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_publish_load() {
        let cell = RcuCell::new(42u32);
        assert_eq!(**cell.load(), 42);
        assert_eq!(cell.version(), 0);

        cell.publish(99);
        assert_eq!(**cell.load(), 99);
        assert_eq!(cell.version(), 1);
    }

    #[test]
    fn old_guard_survives_publish() {
        let cell = RcuCell::new("hello".to_string());
        let old = cell.load();
        cell.publish("world".to_string());
        // old guard must still see "hello" - that's the whole RCU contract
        assert_eq!(&**old, "hello");
        assert_eq!(&**cell.load(), "world");
    }

    #[test]
    fn threaded_reads() {
        let cell = Arc::new(RcuCell::new(0u64));
        let cell2 = Arc::clone(&cell);

        let n = 100_000u64;

        let writer = std::thread::spawn(move || {
            for i in 1..=n {
                cell2.publish(i);
            }
        });

        // reader should never see values going backwards
        let mut last = 0u64;
        loop {
            let val = **cell.load();
            assert!(val >= last, "went backward: {val} < {last}");
            last = val;
            if last == n {
                break;
            }
        }

        writer.join().unwrap();
        assert_eq!(**cell.load(), n);
    }

    #[test]
    fn version_increments() {
        let cell = RcuCell::new(0);
        assert_eq!(cell.version(), 0);
        for i in 1..=10 {
            cell.publish(i);
            assert_eq!(cell.version(), i as u64);
        }
    }

    #[test]
    fn default_impl() {
        let cell: RcuCell<Vec<u8>> = RcuCell::default();
        assert!(cell.load().is_empty());
        assert_eq!(cell.version(), 0);
    }
}
