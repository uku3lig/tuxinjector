# current_time

Returns milliseconds since the Unix epoch (January 1, 1970). Uses the system
clock.

### Example

```lua
tx.bind("F10", function()
    local start = tx.current_time()
    tx.sleep(100)
    local elapsed = tx.current_time() - start
    tx.log("slept for " .. elapsed .. "ms")
end)
```

### Arguments

None

### Return values

  - `ms`: number
