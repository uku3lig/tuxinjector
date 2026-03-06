// Plugin API for tuxinjector
//
// Implement the Plugin trait, slap declare_plugin!() on it, compile as cdylib.
// The host loads .so files from ~/.local/share/tuxinjector/plugins/ via dlopen.
//
// TODO: Also add more comments here for the same reason as before

use std::collections::HashMap;
use std::ffi::{c_char, c_void};

// -- API version --

// Bumped on breaking changes. The loader rejects mismatched versions.
pub const API_VERSION: u32 = 1;

// -- C ABI types --

// Metadata returned by the plugin's register function.
#[repr(C)]
pub struct PluginInfo {
    pub api_version: u32,
    pub name: *const c_char,        // null-terminated
    pub version: *const c_char,     // null-terminated
    pub description: *const c_char, // null-terminated
}

// Read-only context the host passes to plugins each frame
#[repr(C)]
pub struct FrameContext {
    pub screen_width: u32,
    pub screen_height: u32,
    pub viewport_x: f32,
    pub viewport_y: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub current_mode: *const c_char, // null-terminated
    pub game_state: *const c_char,   // null-terminated
    pub frame_number: u64,
    pub delta_time_ms: f32,
}

// An RGBA pixel buffer the plugin wants composited onto the overlay
#[repr(C)]
pub struct OverlaySubmission {
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
    // RGBA pixel data - must stay valid until the next on_frame call
    pub pixels: *const u8,
    pub pixel_len: usize,
    // higher = renders on top, 0 = default layer
    pub depth: i32,
    // 0.0 = invisible, 1.0 = fully opaque
    pub opacity: f32,
}

// Events the host sends to plugins
#[repr(C)]
#[derive(Debug)]
pub enum PluginEvent {
    ConfigReloaded,
    ModeSwitch {
        from: *const c_char,
        to: *const c_char,
    },
    GameStateChanged {
        state: *const c_char,
    },
}

// Key-value setting passed on init (value is JSON)
#[repr(C)]
pub struct PluginSetting {
    pub key: *const c_char,   // null-terminated
    pub value: *const c_char, // null-terminated JSON
}

pub const MAX_SUBMISSIONS: usize = 32;

// The vtable every plugin .so must provide
#[repr(C)]
pub struct PluginVtable {
    // Return 0 on success
    pub init: extern "C" fn(
        plugin: *mut c_void,
        settings: *const PluginSetting,
        settings_len: usize,
    ) -> i32,

    pub destroy: extern "C" fn(plugin: *mut c_void),

    // Fill out_submissions, write count to out_count. Return 0 on success.
    pub on_frame: extern "C" fn(
        plugin: *mut c_void,
        ctx: *const FrameContext,
        out_submissions: *mut OverlaySubmission,
        max_submissions: usize,
        out_count: *mut usize,
    ) -> i32,

    // Plugin can ignore events it doesn't care about
    pub on_event: extern "C" fn(plugin: *mut c_void, event: *const PluginEvent),

    // Return settings schema as JSON. Return nonzero if buffer too small.
    pub get_settings_schema: extern "C" fn(
        plugin: *mut c_void,
        out_schema: *mut u8,
        max_len: usize,
        out_len: *mut usize,
    ) -> i32,
}

// Signature of the `tx_plugin_register` export
pub type PluginRegisterFn = extern "C" fn(
    out_info: *mut PluginInfo,
    out_vtable: *mut PluginVtable,
    out_plugin_data: *mut *mut c_void,
) -> i32;

// -- Safe Rust trait --

// Implement this and use declare_plugin!() to get the C ABI wiring for free
pub trait Plugin: Send {
    fn init(&mut self, settings: &HashMap<String, String>) -> Result<(), String>;
    fn destroy(&mut self);
    fn on_frame(&mut self, ctx: &SafeFrameContext) -> Vec<SafeOverlaySubmission>;
    fn on_event(&mut self, event: SafePluginEvent);

    // Return your settings schema as JSON. See README for the format.
    fn settings_schema(&self) -> &str {
        "{}"
    }
}

// Owned version of FrameContext - no raw pointers to worry about
#[derive(Debug, Clone)]
pub struct SafeFrameContext {
    pub screen_width: u32,
    pub screen_height: u32,
    pub viewport_x: f32,
    pub viewport_y: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub current_mode: String,
    pub game_state: String,
    pub frame_number: u64,
    pub delta_time_ms: f32,
}

// Overlay submission with owned pixel data
pub struct SafeOverlaySubmission {
    pub x: f32,
    pub y: f32,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub depth: i32,
    pub opacity: f32,
}

#[derive(Debug, Clone)]
pub enum SafePluginEvent {
    ConfigReloaded,
    ModeSwitch { from: String, to: String },
    GameStateChanged { state: String },
}

// -- declare_plugin! macro --

// Wire up a Plugin impl to the C ABI entry point.
// Usage: declare_plugin!(MyPlugin, "my-plugin", "0.1.0", "Does cool stuff");
#[macro_export]
macro_rules! declare_plugin {
    ($plugin_type:ty, $name:expr, $version:expr, $desc:expr) => {
        // static c-strings for metadata
        static PLUGIN_NAME: &[u8] = concat!($name, "\0").as_bytes();
        static PLUGIN_VERSION: &[u8] = concat!($version, "\0").as_bytes();
        static PLUGIN_DESC: &[u8] = concat!($desc, "\0").as_bytes();

        extern "C" fn __tx_init(
            plugin: *mut std::ffi::c_void,
            settings: *const $crate::PluginSetting,
            settings_len: usize,
        ) -> i32 {
            let p = unsafe { &mut *(plugin as *mut $plugin_type) };
            let mut map = std::collections::HashMap::new();
            for i in 0..settings_len {
                let s = unsafe { &*settings.add(i) };
                let k = unsafe { std::ffi::CStr::from_ptr(s.key) }
                    .to_string_lossy()
                    .into_owned();
                let v = unsafe { std::ffi::CStr::from_ptr(s.value) }
                    .to_string_lossy()
                    .into_owned();
                map.insert(k, v);
            }
            match $crate::Plugin::init(p, &map) {
                Ok(()) => 0,
                Err(_e) => 1,
            }
        }

        extern "C" fn __tx_destroy(plugin: *mut std::ffi::c_void) {
            let p = unsafe { &mut *(plugin as *mut $plugin_type) };
            $crate::Plugin::destroy(p);
            unsafe { drop(Box::from_raw(plugin as *mut $plugin_type)) };
        }

        // HACK: static mut to keep pixel data alive between on_frame calls.
        // The host reads the pixel pointers after on_frame returns, so we
        // can't let the Vec drop until the next call comes in.
        static mut LAST_SUBMISSIONS: Option<Vec<$crate::SafeOverlaySubmission>> = None;

        extern "C" fn __tx_on_frame(
            plugin: *mut std::ffi::c_void,
            ctx: *const $crate::FrameContext,
            out_submissions: *mut $crate::OverlaySubmission,
            max_submissions: usize,
            out_count: *mut usize,
        ) -> i32 {
            let p = unsafe { &mut *(plugin as *mut $plugin_type) };
            let raw = unsafe { &*ctx };

            let safe_ctx = $crate::SafeFrameContext {
                screen_width: raw.screen_width,
                screen_height: raw.screen_height,
                viewport_x: raw.viewport_x,
                viewport_y: raw.viewport_y,
                viewport_width: raw.viewport_width,
                viewport_height: raw.viewport_height,
                current_mode: unsafe {
                    std::ffi::CStr::from_ptr(raw.current_mode)
                }
                .to_string_lossy()
                .into_owned(),
                game_state: unsafe {
                    std::ffi::CStr::from_ptr(raw.game_state)
                }
                .to_string_lossy()
                .into_owned(),
                frame_number: raw.frame_number,
                delta_time_ms: raw.delta_time_ms,
            };

            let subs = $crate::Plugin::on_frame(p, &safe_ctx);
            let count = subs.len().min(max_submissions);
            unsafe { *out_count = count };

            for (i, sub) in subs.iter().enumerate().take(count) {
                unsafe {
                    let out = &mut *out_submissions.add(i);
                    out.x = sub.x;
                    out.y = sub.y;
                    out.width = sub.width;
                    out.height = sub.height;
                    out.pixels = sub.pixels.as_ptr();
                    out.pixel_len = sub.pixels.len();
                    out.depth = sub.depth;
                    out.opacity = sub.opacity;
                }
            }

            // keep pixel data alive until next call
            unsafe { LAST_SUBMISSIONS = Some(subs) };
            0
        }

        extern "C" fn __tx_on_event(
            plugin: *mut std::ffi::c_void,
            event: *const $crate::PluginEvent,
        ) {
            let p = unsafe { &mut *(plugin as *mut $plugin_type) };
            let raw = unsafe { &*event };

            let safe_ev = match raw {
                $crate::PluginEvent::ConfigReloaded => {
                    $crate::SafePluginEvent::ConfigReloaded
                }
                $crate::PluginEvent::ModeSwitch { from, to } => {
                    $crate::SafePluginEvent::ModeSwitch {
                        from: unsafe { std::ffi::CStr::from_ptr(*from) }
                            .to_string_lossy()
                            .into_owned(),
                        to: unsafe { std::ffi::CStr::from_ptr(*to) }
                            .to_string_lossy()
                            .into_owned(),
                    }
                }
                $crate::PluginEvent::GameStateChanged { state } => {
                    $crate::SafePluginEvent::GameStateChanged {
                        state: unsafe { std::ffi::CStr::from_ptr(*state) }
                            .to_string_lossy()
                            .into_owned(),
                    }
                }
            };

            $crate::Plugin::on_event(p, safe_ev);
        }

        extern "C" fn __tx_get_settings_schema(
            plugin: *mut std::ffi::c_void,
            out_schema: *mut u8,
            max_len: usize,
            out_len: *mut usize,
        ) -> i32 {
            let p = unsafe { &*(plugin as *const $plugin_type) };
            let schema = $crate::Plugin::settings_schema(p);
            let bytes = schema.as_bytes();
            if bytes.len() > max_len {
                unsafe { *out_len = bytes.len() };
                return 1; // buffer too small
            }
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_schema, bytes.len());
                *out_len = bytes.len();
            }
            0
        }

        #[no_mangle]
        pub extern "C" fn tx_plugin_register(
            out_info: *mut $crate::PluginInfo,
            out_vtable: *mut $crate::PluginVtable,
            out_plugin_data: *mut *mut std::ffi::c_void,
        ) -> i32 {
            let plugin = Box::new(<$plugin_type>::default());
            let ptr = Box::into_raw(plugin) as *mut std::ffi::c_void;

            unsafe {
                (*out_info).api_version = $crate::API_VERSION;
                (*out_info).name = PLUGIN_NAME.as_ptr() as *const std::ffi::c_char;
                (*out_info).version = PLUGIN_VERSION.as_ptr() as *const std::ffi::c_char;
                (*out_info).description = PLUGIN_DESC.as_ptr() as *const std::ffi::c_char;

                (*out_vtable).init = __tx_init;
                (*out_vtable).destroy = __tx_destroy;
                (*out_vtable).on_frame = __tx_on_frame;
                (*out_vtable).on_event = __tx_on_event;
                (*out_vtable).get_settings_schema = __tx_get_settings_schema;

                *out_plugin_data = ptr;
            }

            0
        }
    };
}
