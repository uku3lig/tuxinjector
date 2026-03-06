# active_res

Returns the current game resolution in pixels. If no mode resize is active,
this just returns the physical window size.

### Example

```lua
tx.bind("F10", function()
    local w, h = tx.active_res()
    tx.log("game resolution: " .. w .. "x" .. h)
end)
```

### Arguments

None

### Return values

  - `width`: number
  - `height`: number

> This function can be called at any time.
