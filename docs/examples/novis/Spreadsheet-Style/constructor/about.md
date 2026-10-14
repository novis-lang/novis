Makes a `Style` from its two values: the style fields and a key.

`new Style` takes `fields` and `key`, in that order, and each becomes a read-only property. `fields`
has every style field, and a field that is not set is `null`. `key` is a text that is the same for
two styles with equal fields, so the file has one entry for both.

**Good to know:** `Style::of` makes a `Style` for you. It checks each field, turns names such as
`"thin"` and `"center"` into their values, and makes the key. Use `Style::of` in your programs.
