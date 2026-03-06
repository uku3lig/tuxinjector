# tuxinjector

A pure-Rust overlay which injects into Minecraft's rendering pipeline on Linux by using `LD_PRELOAD`. This makes it render directly in the game's OpenGL backbuffer, which easily provides real time resizing and overlays for speedrunning (Or just general use) without any external capture or compositing overhead.

**[Full documentation](https://flammablebunny.github.io/tuxinjector/)**

---

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

For a deeper look at the injection, rendering, and input systems, check the [architecture docs](https://flammablebunny.github.io/tuxinjector/injection/).

---

## Usage

### Prism Launcher

Set a **Wrapper Command** in your instance settings under **Custom Commands**:

```
env LD_PRELOAD=/path/to/tuxinjector.so
```

You can also set `LD_PRELOAD` under the Environment Variables tab instead. See the [usage docs](https://flammablebunny.github.io/tuxinjector/usage/) for full setup instructions.

---

## Configuration

Everything is configured through a single Lua file at `~/.config/tuxinjector/init.lua`. It returns a table with nested sub-configs:

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

The [Lua API reference](https://flammablebunny.github.io/tuxinjector/api/) covers all the scripting functions for keybinds, mode switching, sensitivity, and more.

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

---

## Building

### With Nix

```bash
nix develop
cargo build --release
```

### Without Nix

```bash
# Ensure pkg-config and OpenGL dev headers are installed
cargo build --release
```

Produces `target/release/libtuxinjector.so`.

### Tests

```bash
cargo test  # 148 tests across all crates
```

---

## Thanks

This project would never have been possible without the work of the linux and mcsr communites as a whole, but i would like to give a special thanks to:

- **[tesselslate](https://github.com/tesselslate)** - for [waywall](https://github.com/tesselslate/waywall), which toolscreen was modeled around, and which tux injector's Lua API is based off of.
- **[jojoe77777](https://github.com/jojoe77777)** - for [toolscreen](https://github.com/jojoe77777/ToolScreen), which laid the groundwork and modeled out the idea for what an injection overlay tool should look like, and how it interacts with the game.

And to everyone who tested any early builds of tux injector, which greatly helped find and iron out various bugs from the codebase.
