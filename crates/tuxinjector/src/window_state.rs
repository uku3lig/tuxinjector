// Window title interception -- derives game state from Minecraft title changes.

use std::ffi::{c_char, c_void, CStr};
use std::sync::atomic::{AtomicPtr, Ordering};

type SetWindowTitleFn = unsafe extern "C" fn(window: *mut c_void, title: *const c_char);

static REAL_FN: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

pub fn store_real_set_window_title(ptr: *mut c_void) {
    REAL_FN.store(ptr, Ordering::Release);
}

// "singleplayer" or "multiplayer" in the title -> ingame, otherwise menu
fn title_to_state(title: &str) -> &'static str {
    if title.is_empty() {
        return "";
    }
    let lower = title.to_lowercase();
    if lower.contains("singleplayer") || lower.contains("multiplayer") {
        "ingame"
    } else {
        "menu"
    }
}

pub unsafe extern "C" fn hooked_glfw_set_window_title(
    window: *mut c_void,
    title: *const c_char,
) {
    if !title.is_null() {
        if let Ok(s) = CStr::from_ptr(title).to_str() {
            let state = title_to_state(s);
            let changed = if let Ok(mut guard) = crate::state::get().game_state.lock() {
                if guard.as_str() != state {
                    tracing::debug!(title = s, game_state = state, "game state changed (title)");
                    *guard = state.to_string();
                    true
                } else {
                    false
                }
            } else {
                false
            };

            // don't override if wpstateout already set something more specific
            if changed {
                let cur = tuxinjector_lua::get_game_state();
                let is_title_derived = matches!(cur.as_str(), "" | "menu" | "ingame");
                if is_title_derived && tuxinjector_lua::update_game_state(state) {
                    let tx = crate::state::get();
                    if let Some(rt) = tx.lua_runtime.get() {
                        let _ = rt.state_event_tx.try_send(state.to_string());
                    }
                }
            }
        }
    }

    // forward to real GLFW
    let real = REAL_FN.load(Ordering::Acquire);
    if !real.is_null() {
        let f: SetWindowTitleFn = std::mem::transmute(real);
        f(window, title);
    }
}
