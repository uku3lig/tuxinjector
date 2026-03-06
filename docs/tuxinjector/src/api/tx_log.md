# log

Logs a message to tuxinjector's tracing output. Shows up in the journal or
terminal with the `lua` target.

### Example

```lua
tx.bind("F9", function()
    local w, h = tx.active_res()
    tx.log("current resolution: " .. w .. "x" .. h)
end)
```

### Arguments

  - `message`: string

### Return values

None

> This function can be called at any time.
