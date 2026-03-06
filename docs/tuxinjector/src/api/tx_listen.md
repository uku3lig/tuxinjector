# listen

Registers an event listener that fires whenever the given event happens. Stays
active until the config gets reloaded.

Valid event names:

  - `state`
    - Fires when the game's state changes (title screen to in-world, pause to
      unpaused, etc). The listener gets the new state string as its argument.

### Example

```lua
local tx = require("tuxinjector")

-- Log every state change
tx.listen("state", function(new_state)
    tx.log("state changed to: " .. new_state)
end)

-- Auto-switch mode when entering a world
tx.listen("state", function(s)
    if s == "inworld" then
        tx.switch_mode("Thin")
    end
end)
```

### Arguments

  - `event`: string
  - `listener`: function

### Return values

None

> This function can only be called during config-time.
