use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::{AtomicUsize, Ordering};

// Lock-free SPSC bounded ring buffer. N must be a power of two.
pub struct SpscQueue<T, const N: usize> {
    head: AtomicUsize,
    tail: AtomicUsize,
    slots: [UnsafeCell<MaybeUninit<T>>; N],
}

// SAFETY: one thread owns head (consumer), one owns tail (producer).
// Atomics handle the handoff so slot contents are never raced on.
unsafe impl<T: Send, const N: usize> Send for SpscQueue<T, N> {}
unsafe impl<T: Send, const N: usize> Sync for SpscQueue<T, N> {}

impl<T, const N: usize> SpscQueue<T, N> {
    pub fn new() -> Self {
        assert!(N.is_power_of_two() && N > 0, "N must be a power of two");

        // Can't use [UnsafeCell::new(MaybeUninit::uninit()); N] because
        // UnsafeCell doesn't impl Copy, so we build it the hard way.
        let slots = unsafe {
            let mut arr: MaybeUninit<[UnsafeCell<MaybeUninit<T>>; N]> = MaybeUninit::uninit();
            let base = arr.as_mut_ptr() as *mut UnsafeCell<MaybeUninit<T>>;
            for i in 0..N {
                base.add(i).write(UnsafeCell::new(MaybeUninit::uninit()));
            }
            arr.assume_init()
        };

        Self {
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            slots,
        }
    }

    #[inline]
    fn mask(idx: usize) -> usize {
        idx & (N - 1)
    }

    // Push a value into the queue. Returns Err(val) if full.
    // Only call from the producer thread!
    pub fn try_push(&self, val: T) -> Result<(), T> {
        let tail = self.tail.load(Ordering::Relaxed);
        let next = tail.wrapping_add(1);

        if Self::mask(next) == Self::mask(self.head.load(Ordering::Acquire)) {
            return Err(val); // full
        }

        unsafe {
            (*self.slots[Self::mask(tail)].get()).write(val);
        }
        self.tail.store(next, Ordering::Release);
        Ok(())
    }

    // Pop from the queue. Returns None if empty.
    // Only call from the consumer thread!
    pub fn try_pop(&self) -> Option<T> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if Self::mask(head) == Self::mask(tail) && head == tail {
            return None;
        }

        let val = unsafe { (*self.slots[Self::mask(head)].get()).assume_init_read() };
        self.head.store(head.wrapping_add(1), Ordering::Release);
        Some(val)
    }

    pub fn is_empty(&self) -> bool {
        let h = self.head.load(Ordering::Relaxed);
        let t = self.tail.load(Ordering::Relaxed);
        h == t
    }

    // NOTE: this is approximate - can be slightly off if producer/consumer are racing
    pub fn len(&self) -> usize {
        let h = self.head.load(Ordering::Relaxed);
        let t = self.tail.load(Ordering::Relaxed);
        t.wrapping_sub(h)
    }
}

impl<T, const N: usize> Default for SpscQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> Drop for SpscQueue<T, N> {
    fn drop(&mut self) {
        // drain anything left so we don't leak
        while self.try_pop().is_some() {}
    }
}

impl<T, const N: usize> std::fmt::Debug for SpscQueue<T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpscQueue")
            .field("len", &self.len())
            .field("capacity", &(N - 1))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn push_pop_basic() {
        let q = SpscQueue::<u32, 4>::new();
        assert!(q.is_empty());

        assert!(q.try_push(1).is_ok());
        assert!(q.try_push(2).is_ok());
        assert!(q.try_push(3).is_ok());
        // power-of-two ring wastes one slot for full/empty disambiguation
        assert!(q.try_push(4).is_err());

        assert_eq!(q.try_pop(), Some(1));
        assert_eq!(q.try_pop(), Some(2));
        assert_eq!(q.try_pop(), Some(3));
        assert_eq!(q.try_pop(), None);
    }

    #[test]
    fn wraparound() {
        let q = SpscQueue::<u32, 4>::new();
        for i in 0..10 {
            assert!(q.try_push(i).is_ok());
            assert_eq!(q.try_pop(), Some(i));
        }
    }

    #[test]
    fn threaded_transfer() {
        let q = Arc::new(SpscQueue::<u64, 256>::new());
        let q2 = Arc::clone(&q);

        let count = 100_000u64;

        let producer = std::thread::spawn(move || {
            for i in 0..count {
                while q2.try_push(i).is_err() {
                    std::hint::spin_loop();
                }
            }
        });

        let mut got = Vec::with_capacity(count as usize);
        while got.len() < count as usize {
            if let Some(v) = q.try_pop() {
                got.push(v);
            }
        }

        producer.join().unwrap();

        for (i, &v) in got.iter().enumerate() {
            assert_eq!(v, i as u64, "mismatch at index {i}");
        }
    }

    #[test]
    #[should_panic]
    fn non_power_of_two_panics() {
        let _ = SpscQueue::<u32, 3>::new();
    }

    #[test]
    fn drop_remaining() {
        let q = SpscQueue::<String, 4>::new();
        q.try_push("hello".to_string()).unwrap();
        q.try_push("world".to_string()).unwrap();
        drop(q); // should not leak
    }
}
