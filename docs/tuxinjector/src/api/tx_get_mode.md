# get_mode

Returns the name of the currently active display mode.

### Example

```lua
tx.bind("F10", function()
    local mode = tx.get_mode()
    tx.log("current mode: " .. mode)
end)
```

### Arguments

None

### Return values

  - `mode`: string

> This function can be called at any time.
