# toggle_gui

Toggles the settings GUI overlay. When the GUI opens during gameplay, the
cursor is forced visible. When closed, the cursor goes back to its previous
state.

### Example

```lua
tx.bind("ctrl+I", function()
    tx.toggle_gui()
end)
```

### Arguments

None

### Return values

None

> This function cannot be called during config-time.
