# tx

The `tx` module contains all functions for interacting with tuxinjector. It is
loaded via `require("tuxinjector")`:

```lua
local tx = require("tuxinjector")
```

Some functions are **config-time** only (when the Lua file is first evaluated),
others are **runtime** only (inside keybind callbacks or event listeners). Each
function's page says which.
