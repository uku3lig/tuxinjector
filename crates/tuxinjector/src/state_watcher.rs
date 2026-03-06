// Polls wpstateout.txt for game state detection (wall/title/inworld).
// Fires Lua ts.listen("state", fn) events on changes.
//
// NOTE: "Using wpstateout.txt (State Output and previously WorldPreview), state.json (Hermes), record.json (SpeedRunIGT), or other mod-outputted instance state as a performant replacement for checks possible in the unmodified game is permitted" (a.8.14.a)

use std::path::PathBuf;
use std::time::Duration;

const POLL_MS: Duration = Duration::from_millis(50);

fn find_state_file() -> Option<PathBuf> {
    // explicit override
    if let Ok(p) = std::env::var("TUXINJECTOR_STATE_FILE") {
        let path = PathBuf::from(p);
        if path.exists() { return Some(path); }
    }

    // XDG config
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        let p = PathBuf::from(xdg).join("tuxinjector/wpstateout.txt");
        if p.exists() { return Some(p); }
    }

    // fallback
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".config/tuxinjector/wpstateout.txt");
        if p.exists() { return Some(p); }
    }

    None
}

// Map raw wpstateout line to a canonical state string.
// "inworld" is special because we also check cursor grab state.
fn to_game_state(raw: &str) -> String {
    let tag = raw.split(',').next().unwrap_or("").trim();
    match tag {
        "wall"                      => "wall".into(),
        "title"                     => "title".into(),
        "waiting"                   => "waiting".into(),
        "generating" | "previewing" => "generating".into(),
        "inworld" => {
            if tuxinjector_input::is_cursor_captured() {
                "inworld,cursor_grabbed".into()
            } else {
                "inworld,cursor_free".into()
            }
        }
        _ => "title".into(),
    }
}

pub fn spawn_state_watcher() {
    let _ = std::thread::Builder::new()
        .name("state-watcher".into())
        .spawn(watcher_loop);
}

fn watcher_loop() {
    let mut last_raw = String::new();
    let mut last_state = String::new();
    let mut path: Option<PathBuf> = None;
    let mut found_logged = false;

    loop {
        // keep looking for the file if we don't have it yet
        if path.is_none() {
            path = find_state_file();
            if let Some(ref p) = path {
                if !found_logged {
                    tracing::info!(path = %p.display(), "state watcher: found wpstateout.txt");
                    found_logged = true;
                }
            }
        }

        if let Some(ref p) = path {
            match std::fs::read_to_string(p) {
                Ok(content) => {
                    last_raw = content.trim().to_string();
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // file disappeared, go back to searching
                    path = None;
                    found_logged = false;
                    last_raw.clear();
                }
                Err(e) => {
                    tracing::warn!(error = %e, "state watcher: read error");
                }
            }
        }

        // recompute every tick so cursor-mode changes get picked up
        if !last_raw.is_empty() {
            let state = to_game_state(&last_raw);
            if state != last_state {
                last_state = state.clone();
                tracing::debug!(raw = %last_raw, state = %state, "wpstateout state change");
                push_state(&state);
            }
        }

        std::thread::sleep(POLL_MS);
    }
}

fn push_state(state: &str) {
    let tx = crate::state::get();

    // update hotkey engine's condition state
    if let Ok(mut guard) = tx.game_state.lock() {
        *guard = state.to_string();
    }

    // lua shared state + runtime notification
    if tuxinjector_lua::update_game_state(state) {
        if let Some(rt) = tx.lua_runtime.get() {
            let _ = rt.state_event_tx.try_send(state.to_string());
        }
    }
}
