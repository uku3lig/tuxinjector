# set_sensitivity

Sets the mouse sensitivity multiplier for camera movement. Pass 0 to reset
back to whatever you have in `input.mouseSensitivity`.

### Example

```lua
-- Halve sensitivity when switching to Tall mode
tx.bind("J", function()
    tx.switch_mode("Tall")
    tx.set_sensitivity(0.5)
end)

-- Reset to config default
tx.bind("ctrl+J", function()
    tx.switch_mode("Fullscreen")
    tx.set_sensitivity(0)
end)
```

### Arguments

  - `sensitivity`: number

### Return values

None

> This function cannot be called during config-time.
