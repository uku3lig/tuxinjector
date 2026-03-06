// This file was also heavily organised by an LLM
//
// ═══════════════════════════════════════════════════════════════════════════
// Module: glfw_types — GLFW Type Definitions & Constants
// ═══════════════════════════════════════════════════════════════════════════
//
// Hand-rolled subset of glfw3.h, translated to Rust types for
// callback interception.

use std::ffi::c_void;

// ── Action Constants ──
pub const GLFW_RELEASE: i32 = 0;
pub const GLFW_PRESS: i32 = 1;
pub const GLFW_REPEAT: i32 = 2;

// ── Key Constants ──
pub const GLFW_KEY_ESCAPE: i32 = 256;
pub const GLFW_KEY_ENTER: i32 = 257;
pub const GLFW_KEY_TAB: i32 = 258;
pub const GLFW_KEY_BACKSPACE: i32 = 259;
pub const GLFW_KEY_INSERT: i32 = 260;
pub const GLFW_KEY_DELETE: i32 = 261;

pub const GLFW_KEY_F1: i32 = 290;
pub const GLFW_KEY_F2: i32 = 291;
pub const GLFW_KEY_F3: i32 = 292;
pub const GLFW_KEY_F4: i32 = 293;
pub const GLFW_KEY_F5: i32 = 294;
pub const GLFW_KEY_F6: i32 = 295;
pub const GLFW_KEY_F7: i32 = 296;
pub const GLFW_KEY_F8: i32 = 297;
pub const GLFW_KEY_F9: i32 = 298;
pub const GLFW_KEY_F10: i32 = 299;
pub const GLFW_KEY_F11: i32 = 300;
pub const GLFW_KEY_F12: i32 = 301;

pub const GLFW_KEY_LEFT_SHIFT: i32 = 340;
pub const GLFW_KEY_LEFT_CONTROL: i32 = 341;
pub const GLFW_KEY_LEFT_ALT: i32 = 342;
pub const GLFW_KEY_LEFT_SUPER: i32 = 343;
pub const GLFW_KEY_RIGHT_SHIFT: i32 = 344;
pub const GLFW_KEY_RIGHT_CONTROL: i32 = 345;
pub const GLFW_KEY_RIGHT_ALT: i32 = 346;
pub const GLFW_KEY_RIGHT_SUPER: i32 = 347;

// ── Modifier Bitmask ──
pub const GLFW_MOD_SHIFT: i32 = 0x0001;
pub const GLFW_MOD_CONTROL: i32 = 0x0002;
pub const GLFW_MOD_ALT: i32 = 0x0004;
pub const GLFW_MOD_SUPER: i32 = 0x0008;

// ── Opaque GLFWwindow Handle ──
pub type GlfwWindow = *mut c_void;

// ── Callback Signatures (mirrors GLFW's typedefs) ──

pub type GlfwKeyCallback = Option<
    unsafe extern "C" fn(window: GlfwWindow, key: i32, scancode: i32, action: i32, mods: i32),
>;

pub type GlfwMouseButtonCallback =
    Option<unsafe extern "C" fn(window: GlfwWindow, button: i32, action: i32, mods: i32)>;

pub type GlfwCursorPosCallback =
    Option<unsafe extern "C" fn(window: GlfwWindow, xpos: f64, ypos: f64)>;

pub type GlfwScrollCallback =
    Option<unsafe extern "C" fn(window: GlfwWindow, xoffset: f64, yoffset: f64)>;

pub type GlfwCharCallback = Option<unsafe extern "C" fn(window: GlfwWindow, codepoint: u32)>;

pub type GlfwCharModsCallback =
    Option<unsafe extern "C" fn(window: GlfwWindow, codepoint: u32, mods: i32)>;

// ── glfwSetXxx Function Pointer Types ──

pub type GlfwSetKeyCallbackFn =
    unsafe extern "C" fn(window: GlfwWindow, callback: GlfwKeyCallback) -> GlfwKeyCallback;

pub type GlfwSetMouseButtonCallbackFn = unsafe extern "C" fn(
    window: GlfwWindow,
    callback: GlfwMouseButtonCallback,
) -> GlfwMouseButtonCallback;

pub type GlfwSetCursorPosCallbackFn =
    unsafe extern "C" fn(window: GlfwWindow, callback: GlfwCursorPosCallback)
        -> GlfwCursorPosCallback;

pub type GlfwSetScrollCallbackFn =
    unsafe extern "C" fn(window: GlfwWindow, callback: GlfwScrollCallback) -> GlfwScrollCallback;

pub type GlfwSetCharCallbackFn =
    unsafe extern "C" fn(window: GlfwWindow, callback: GlfwCharCallback) -> GlfwCharCallback;

pub type GlfwSetCharModsCallbackFn = unsafe extern "C" fn(
    window: GlfwWindow,
    callback: GlfwCharModsCallback,
) -> GlfwCharModsCallback;

pub type GlfwSetInputModeFn = unsafe extern "C" fn(window: GlfwWindow, mode: i32, value: i32);

// ── Mouse Button Constants ──
pub const GLFW_MOUSE_BUTTON_1: i32 = 0; // left
pub const GLFW_MOUSE_BUTTON_2: i32 = 1; // right
pub const GLFW_MOUSE_BUTTON_3: i32 = 2; // middle
pub const GLFW_MOUSE_BUTTON_4: i32 = 3;
pub const GLFW_MOUSE_BUTTON_5: i32 = 4;
pub const GLFW_MOUSE_BUTTON_6: i32 = 5;
pub const GLFW_MOUSE_BUTTON_7: i32 = 6;
pub const GLFW_MOUSE_BUTTON_8: i32 = 7;

// Mouse buttons are encoded at offset 400+ in keycode space, permitting
// them to share the same rebind/hotkey tables as keyboard keys.
pub const MOUSE_BUTTON_OFFSET: i32 = 400;

// ── Input Mode Constants ──
pub const GLFW_CURSOR: i32 = 0x00033001;
pub const GLFW_CURSOR_DISABLED: i32 = 0x00034003; // FPS mode
pub const GLFW_CURSOR_NORMAL: i32 = 0x00034001;
