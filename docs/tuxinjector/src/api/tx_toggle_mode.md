# toggle_mode

Toggles between two display modes. If the current mode is `main`, switches to
`fallback`. Otherwise (including if you're already on `fallback`), switches to
`main`.

The mode is resolved at dispatch time on the GL thread, so rapid double-presses
correctly reverse direction even mid-transition.

### Example

```lua
-- Toggle between Fullscreen and Thin on Z
tx.bind("Z", function()
    tx.toggle_mode("Fullscreen", "Thin")
end)
```

### Arguments

  - `main`: string
  - `fallback`: string

### Return values

None

> This function cannot be called during config-time.
