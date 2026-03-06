# bind

Registers a keybind - runs the callback whenever the key combo is pressed.
Combos are `+`-separated strings of [key names](key_names.md),
case-insensitive.

You can also pass an `options` table:

  - `block` (boolean, default `true`): If `true`, the key event won't reach
    the game. Set `false` if you want the game to see the press too.

### Example

```lua
local tx = require("tuxinjector")

-- Switch to Thin mode on Ctrl+F1 (key is blocked from game)
tx.bind("ctrl+F1", function()
    tx.switch_mode("Thin")
end)

-- Toggle GUI on F2 (key is also passed to game)
tx.bind("F2", function()
    tx.toggle_gui()
end, { block = false })

-- Multi-key combo
tx.bind("ctrl+shift+Z", function()
    tx.log("ctrl+shift+z pressed")
end)
```

### Arguments

  - `keys`: string
  - `callback`: function
  - `options`: table (optional)

### Return values

None

> This function can only be called during config-time (top-level execution).
> Calling it inside a keybind callback will have no effect until the next
> config reload.
