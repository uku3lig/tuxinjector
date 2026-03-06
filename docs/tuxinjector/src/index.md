# Introduction

**Docs Version:** 1.0
**Project:** Tuxinjector - Injection based minecraft speedrunning tool 
**Stack:** Rust / OpenGL (GLSL 300 ES) / GLFW interception / Lua config / imgui-rs

---

## What is Tuxinjector?

Tuxinjector is a pure-Rust overlay which injects into Minecraft's rendering pipeline on Linux by using `LD_PRELOAD`. This makes it render directly in the game's OpenGL backbuffer, which easily provides real time resizing and overlays for speedrunning (Or just general use) without any external capture or compositing overhead.
<!-- TODO: Update this for MacOs Implementation, which should be relatively soon. --> 
## How It Works

Tuxinjector is compiled into a shared object (`tuxinjector.so`) which gets loaded via `LD_PRELOAD` before the game starts. When the game's JVM calls `dlsym` to look up OpenGL and GLFW functions, tuxinjector intercepts those lookups and returns its own wrapper functions. The wrappers stash the real function pointers and add overlay logic before/after forwarding to the originals.

```
Game launch:
  LD_PRELOAD=tuxinjector.so minecraft

1. Game's JVM loads -> dlsym("eglSwapBuffers") -> tuxinjector's hooked dlsym
2. Hooked dlsym: stash real eglSwapBuffers, return hooked_egl_swap_buffers
3. Every frame: game calls hooked_egl_swap_buffers
4. Hook: render_overlay() -> draw scene into backbuffer -> call real eglSwapBuffers
5. Buffer is presented with overlay composited on top
```

Input works the same way - `dlsym("glfwSetKeyCallback")` gets intercepted, the game's callback is stashed, and our wrapper gets installed instead. The wrapper handles hotkeys and key rebinds before forwarding events to the game.

---

## Crate Structure

Tuxinjector is set up as a Rust workspace split up into 10 different crates. Splitting things up helps keep compile times low, and also makes it easier to isolate some of the unsafe GL stuff.

| Crate | Purpose |
|-------|---------|
| `tuxinjector` | Main library: hooks, overlay state, mode system, plugin loader |
| `tuxinjector-core` | Shared types: Color, geometry, lock-free primitives (RCU, SPSC) |
| `tuxinjector-config` | Config types, Lua hot-reload, serde defaults |
| `tuxinjector-input` | GLFW callback interception, key rebinding, sensitivity scaling |
| `tuxinjector-render` | Shader pipeline, texture management, image loading |
| `tuxinjector-gl-interop` | Direct GL renderer, GL state save/restore, scene compositor |
| `tuxinjector-gui` | imgui-rs settings UI (14 tabs), toast notifications |
| `tuxinjector-lua` | Lua scripting runtime, hotkey actions, config loader |
| `tuxinjector-capture` | Mirror capture (FBO readback, PBO async, zero-copy texture ref) |
| `tuxinjector-plugin-api` | C ABI plugin trait, `declare_plugin!` macro |

The split isn't perfect yet - a couple things are probably in misleading places, but it works and thats all that really matters :)

---

## Configuration

Everything is configured through a single Lua file at `~/.config/tuxinjector/init.lua`. It returns a table with nested sub-configs:
<!-- Does macos have this directory? --> 
```lua
return {
    display = {
        defaultMode = "Fullscreen",
        fpsLimit = 0,
    },
    input = {
        mouseSensitivity = 1.0,
        keyRebinds = { enabled = true, rebinds = { ... } },
    },
    theme = {
        fontPath = "/usr/share/fonts/truetype/DejaVuSans.ttf",
        appearance = { theme = "Purple", guiScale = 0.8 },
    },
    overlays = {
        mirrors = { ... },
        images = { ... },
    },
    modes = { ... },
}
```

Hot-reload is supported - editing `init.lua` while the game is running applies changes immediately without needing to restart.
