// Toast notification queue. GUI renderer drains these once per frame.
// Mutex is overkill for a handful of strings but it works fine here.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

pub struct Pending {
    pub message: String,
    pub color: Option<[u8; 4]>,
}

fn queue() -> &'static Mutex<VecDeque<Pending>> {
    static Q: OnceLock<Mutex<VecDeque<Pending>>> = OnceLock::new();
    Q.get_or_init(|| Mutex::new(VecDeque::new()))
}

pub fn push(msg: impl Into<String>) {
    if let Ok(mut q) = queue().lock() {
        q.push_back(Pending {
            message: msg.into(),
            color: None,
        });
    }
}

pub fn push_colored(msg: impl Into<String>, rgba: [u8; 4]) {
    if let Ok(mut q) = queue().lock() {
        q.push_back(Pending {
            message: msg.into(),
            color: Some(rgba),
        });
    }
}

pub fn drain() -> Vec<Pending> {
    match queue().lock() {
        Ok(mut q) => q.drain(..).collect(),
        Err(_) => vec![],
    }
}
