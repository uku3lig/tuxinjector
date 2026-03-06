# Injection & Hooking <!-- TODO: Update ALL of this for MacOs Implementation -->

Tuxinjector uses `LD_PRELOAD` to inject directly into the game process before any game code runs. It works by hooking `dlsym` and `dlopen` to intercept symbol lookups, and also exports PLT-level overrides for GLFW stuff since LWJGL3 uses `RTLD_DEEPBIND` which would otherwise bypass our hooks. All of this gives us full control over GL rendering and input without touching any game files.

---

## Interception Strategy

Minecraft (LWJGL3) resolves its GL and GLFW functions through two different paths, and we have to handle both:

| Resolution Path | Used By | Interception Method |
|----------------|---------|---------------------|
| `dlsym(RTLD_NEXT, ...)` | EGL/GLX swap, GL functions | Hooked `dlsym` via `dlvsym` |
| `dlopen` + PLT binding | GLFW functions (with `RTLD_DEEPBIND`) | `#[no_mangle]` PLT exports + `dlopen` hook |

LWJGL3 loads `libglfw.so` with `RTLD_DEEPBIND`, which creates a private symbol scope that completely bypasses our `dlsym` hook. The workaround is exporting `#[no_mangle]` symbols at the PLT level, so the linker resolves those before `RTLD_DEEPBIND` can do anything about it.

---

## dlsym Hook

The core of everything is the `dlsym` hook. Since we've interposed `dlsym` itself, we need a way to call the *real* one without recursing into ourselves - `dlvsym` (versioned symbol lookup) handles that.

### Resolving the Real dlsym

```rust
dlsym {
    resolve_real_dlsym: DlsymFn    // Resolved via dlvsym(RTLD_NEXT, "dlsym", "GLIBC_2.34")
}
```

Tries `GLIBC_2.34` first, falls back to `GLIBC_2.2.5` for older glibc.

### Intercepted Symbols

So when the game does `dlsym(handle, "eglSwapBuffers")`, our hook:

1. Calls the real `dlsym` to get the actual function pointer
2. Stashes that pointer in an `AtomicPtr` for later
3. Returns our wrapper function instead

```
dlsym("eglSwapBuffers")  ->  stash real ptr  ->  return hooked_egl_swap_buffers
dlsym("glXSwapBuffers")  ->  stash real ptr  ->  return hooked_glx_swap_buffers
dlsym("glfwSetKeyCallback")  ->  stash real ptr  ->  return hooked_set_key_callback
dlsym("glfwGetKey")  ->  stash real ptr  ->  return glfwGetKey (PLT export)
dlsym("glViewport")  ->  stash real ptr  ->  return glViewport (viewport hook)
...
```

Full list of everything we intercept:

| Category | Symbols |
|----------|---------|
| **Swap** | `eglSwapBuffers`, `glXSwapBuffers`, `eglGetProcAddress`, `glXGetProcAddressARB` |
| **GLFW callbacks** | `glfwSetKeyCallback`, `glfwSetMouseButtonCallback`, `glfwSetCursorPosCallback`, `glfwSetScrollCallback`, `glfwSetCharCallback`, `glfwSetCharModsCallback`, `glfwSetInputMode`, `glfwSetFramebufferSizeCallback` |
| **GLFW polling** | `glfwGetKey`, `glfwGetMouseButton`, `glfwGetCursorPos`, `glfwGetFramebufferSize`, `glfwGetProcAddress` |
| **GL functions** | `glViewport`, `glScissor`, `glBindFramebuffer` (+ EXT/ARB), `glDrawBuffer`, `glReadBuffer`, `glDrawBuffers`, `glBlitFramebuffer` |

---

## dlopen Hook

LWJGL3 loads its JNI libraries with `RTLD_DEEPBIND`, which would hide our PLT exports. Pretty simple fix - just strip `RTLD_DEEPBIND` from the flags before forwarding to the real `dlopen`:

```
dlopen(path, flags):
    clean_flags = flags & ~RTLD_DEEPBIND
    return real_dlopen(path, clean_flags)
```

Without `RTLD_DEEPBIND`, LWJGL3's GLFW JNI bindings resolve from the global namespace where our `#[no_mangle]` exports are.

---

## PLT-Level Exports

GLFW functions that get resolved through direct PLT binding (not `dlsym`) need separate `#[no_mangle]` exports with the same name and ABI:

```rust
#[no_mangle]
pub unsafe extern "C" fn glfwSetKeyCallback(
    window: GlfwWindow,
    callback: GlfwKeyCallback,
) -> GlfwKeyCallback {
    // Resolve real glfwSetKeyCallback via RTLD_NEXT (once)
    // Intercept: stash game callback, install our wrapper
    callbacks::intercept_set_key_callback(window, callback)
}
```

Each export resolves the real function via `libc::dlsym(RTLD_NEXT, ...)` on first call, which also routes through our hooked `dlsym` to store the real pointer as a side-effect.

---

## glfwGetProcAddress Hook

Minecraft uses `glfwGetProcAddress` to resolve GL function pointers at runtime. We intercept this to hook GL functions that aren't reachable through `dlsym`:

```
Game: glfwGetProcAddress("glViewport")
Hook: call real glfwGetProcAddress("glViewport") -> store real ptr
      return glViewport hook function

Game: glfwGetProcAddress("glBindFramebuffer")
Hook: call real -> store real ptr -> return hook

Game: glfwGetProcAddress("anything_else")
Hook: forward to real unchanged
```

Covers `glViewport`, `glScissor`, `glBindFramebuffer` (and EXT/ARB variants), `glDrawBuffer`, `glReadBuffer`, `glDrawBuffers`, and `glBlitFramebuffer`.

---

## Hook Chaining

When multiple `LD_PRELOAD` libraries hook the same symbols, there are two forwarding modes:

| Mode | Behavior |
|------|----------|
| **Original function** (default) | Resolve the real function directly from the driver library (`libEGL.so`, `libGLX.so`) via `RTLD_NOLOAD`, bypassing other hooks |
| **RTLD_NEXT** | Forward to the next hook in the `LD_PRELOAD` chain |

Original function mode is preferred since it sidesteps compatibility issues with other overlays (MangoHud, etc.) that might also hook swap functions. Configurable via `advanced.disable_hook_chaining`.

---

## First-Frame Initialisation

All the heavy init is deferred to the first frame, when the GL context is actually current. Before that point there's no GL context, so creating shaders/textures/FBOs would just fail.

```
First eglSwapBuffers/glXSwapBuffers call:
    1. Resolve GL function pointers via eglGetProcAddress/glXGetProcAddressARB
    2. Create GlOverlayRenderer (compile shaders, allocate FBOs)
    3. Load config from ~/.config/tuxinjector/init.lua
    4. Register input handler with hotkey engine
    5. Discover and load plugins from ~/.local/share/tuxinjector/plugins/
    6. Install inline glViewport/glBindFramebuffer hooks (runtime patching)
    7. INITIALIZED = true
```

Every subsequent swap call checks `INITIALIZED` before rendering. If init fails, the game keeps running normally without the overlay.

---

## Function Pointer Storage

All real function pointers live in `AtomicPtr<c_void>` statics with `Ordering::Release` on store and `Ordering::Acquire` on load, so they're guaranteed visible across threads.

```rust
static REAL_EGL_SWAP: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

pub fn store_real_egl_swap(ptr: *mut c_void) {
    REAL_EGL_SWAP.store(ptr, Ordering::Release);
}

// In the hooked function:
let ptr = REAL_EGL_SWAP.load(Ordering::Acquire);
let real_fn: EglSwapBuffersFn = std::mem::transmute(ptr);
real_fn(display, surface)
```

Same pattern for every hooked function. The `AtomicPtr` makes sure pointers stored on the game's main thread are safely readable from the render thread.
