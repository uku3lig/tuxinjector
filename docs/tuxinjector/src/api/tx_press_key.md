# press_key

Sends a fake key press (keydown then keyup) to Minecraft. See
[Key Names](key_names.md) for valid names.

If you pass a combo (e.g. `"ctrl+F3"`), each key gets pressed and released
one at a time.

### Example

```lua
-- Send F3+C to the game (copy coordinates)
tx.bind("ctrl+C", function()
    tx.press_key("F3")
    tx.press_key("C")
end)

-- Toggle F3 overlay
tx.bind("ctrl+D", function()
    tx.press_key("F3")
end)
```

### Arguments

  - `key`: string

### Return values

None

> This function cannot be called during config-time.
