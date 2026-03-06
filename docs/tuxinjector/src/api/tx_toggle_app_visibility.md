# toggle_app_visibility

Toggles visibility of anchored companion app windows - the external apps
you spawn via [`tx.exec()`](tx_exec.md) or set up in the Apps tab that get
reparented into the game window.

When hidden, the windows are unmapped but keep running. Toggling back shows
them again at their previous position.

### Example

```lua
-- Hide/show Ninjabrain Bot overlay with a keybind
tx.bind("ctrl+B", function()
    tx.toggle_app_visibility()
end)
```

### Arguments

None

### Return values

None

> This function cannot be called during config-time.
