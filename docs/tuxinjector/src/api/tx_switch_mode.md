# switch_mode

Switches to a display mode by name. It has to exist in the `modes` table
of your config. If the mode has a transition animation set up, the switch
will be animated.

### Example

```lua
tx.bind("Z", function()
    tx.switch_mode("Thin")
end)
```

### Arguments

  - `name`: string

### Return values

None

> This function cannot be called during config-time.
