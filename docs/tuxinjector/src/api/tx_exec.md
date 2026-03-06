# exec

Runs the given command as an async subprocess. The command is passed to
`sh -c`, so shell features (pipes, redirects, variable expansion) all work.

The process gets registered as an anchored companion app, and if it has a
window it'll show up in the Running Apps list.

If it's still running when tuxinjector unloads, it stays running.

### Example

```lua
-- Launch Ninjabrain Bot on a keybind
tx.bind("ctrl+N", function()
    tx.exec("java -jar ~/NinjabrainBot.jar")
end)

-- Open a terminal
tx.bind("ctrl+T", function()
    tx.exec("foot")
end)
```

### Arguments

  - `command`: string

### Return values

None

> This function cannot be called during config-time.
