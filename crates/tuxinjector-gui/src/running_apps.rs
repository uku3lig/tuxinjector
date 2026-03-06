// Tracks companion apps launched from the Apps tab.
// Just a Vec behind a mutex. Keeps it simple.

use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    pub const ALL: &[Anchor] = &[
        Anchor::TopLeft,
        Anchor::Top,
        Anchor::TopRight,
        Anchor::Left,
        Anchor::Center,
        Anchor::Right,
        Anchor::BottomLeft,
        Anchor::Bottom,
        Anchor::BottomRight,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Anchor::TopLeft     => "Top Left",
            Anchor::Top         => "Top",
            Anchor::TopRight    => "Top Right",
            Anchor::Left        => "Left",
            Anchor::Center      => "Center",
            Anchor::Right       => "Right",
            Anchor::BottomLeft  => "Bottom Left",
            Anchor::Bottom      => "Bottom",
            Anchor::BottomRight => "Bottom Right",
        }
    }

    pub fn position(self, vp_w: i32, vp_h: i32, win_w: i32, win_h: i32, margin: i32) -> (i32, i32) {
        let cx = (vp_w - win_w) / 2;
        let cy = (vp_h - win_h) / 2;
        match self {
            Anchor::TopLeft     => (margin, margin),
            Anchor::Top         => (cx, margin),
            Anchor::TopRight    => (vp_w - win_w - margin, margin),
            Anchor::Left        => (margin, cy),
            Anchor::Center      => (cx, cy),
            Anchor::Right       => (vp_w - win_w - margin, cy),
            Anchor::BottomLeft  => (margin, vp_h - win_h - margin),
            Anchor::Bottom      => (cx, vp_h - win_h - margin),
            Anchor::BottomRight => (vp_w - win_w - margin, vp_h - win_h - margin),
        }
    }
}

// TODO: maybe add custom anchors to the function above

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchMode {
    Anchored(Anchor),
    Detached,
}

#[derive(Clone, Debug)]
pub struct RunningApp {
    pub pid: u32,
    pub name: String,
    pub mode: LaunchMode,
}

fn registry() -> &'static Mutex<Vec<RunningApp>> {
    static REG: OnceLock<Mutex<Vec<RunningApp>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn register(pid: u32, name: impl Into<String>, mode: LaunchMode) {
    if let Ok(mut list) = registry().lock() {
        list.retain(|a| a.pid != pid);
        list.push(RunningApp { pid, name: name.into(), mode });
    }
}

pub fn unregister(pid: u32) {
    if let Ok(mut list) = registry().lock() {
        list.retain(|a| a.pid != pid);
    }
}

pub fn list() -> Vec<RunningApp> {
    registry().lock().map(|g| g.clone()).unwrap_or_default()
}
