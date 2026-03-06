//! Atomic config snapshot, shareable across threads

use crate::types::Config;
use tuxinjector_core::rcu::RcuCell;

/// Config snapshot backed by an RCU cell for lock-free reads
pub type ConfigSnapshot = RcuCell<Config>;
