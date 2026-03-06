# get_key

Returns whether a key is currently held down. See [Key Names](key_names.md) for
valid names.

If you pass a combo with multiple keys (e.g. `"ctrl+shift"`), all of them need
to be held for it to return `true`.

### Example

```lua
tx.bind("Z", function()
    if tx.get_key("shift") then
        tx.switch_mode("Wide")
    else
        tx.switch_mode("Thin")
    end
end)
```

### Arguments

  - `key`: string

### Return values

  - `pressed`: boolean

> This function can be called at any time.
