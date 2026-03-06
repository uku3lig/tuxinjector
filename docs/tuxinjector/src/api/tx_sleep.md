# sleep

Pauses the Lua thread for the given number of milliseconds. This blocks the
whole VM thread, so other keybind callbacks and listeners get delayed until
it's done.

### Example

```lua
-- Press F3, wait 50ms, then press S (to open the pie chart)
tx.bind("ctrl+P", function()
    tx.press_key("F3")
    tx.sleep(50)
    tx.press_key("S")
end)
```

### Arguments

  - `ms`: number

### Return values

None

> This function cannot be called during config-time.
