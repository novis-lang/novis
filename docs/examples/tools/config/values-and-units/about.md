A value in `nvs.toml` is a number with a unit, and the key decides which unit the value has.

There are four units. A size is a whole number with `K`, `M`, `G` or `T`, and a number without a
letter is bytes. Upper case and lower case are the same. A duration is a whole number with `ns`,
`us`, `ms`, `s`, `m`, `h` or `d`, and a number without a letter is seconds. A count is a whole
number. A ratio is a number from `0` to `1`.

The same letter can mean two things. `m` means mebibytes for `memory` and minutes for `wall_time`.

A value has one number and one suffix. `90` and `2m` are valid durations. `1m30s` and `1.5m` are
not valid. `false` for a ceiling key means that there is no ceiling.

**Good to know:** `nvs run` and `nvs config check` do not report a value that is not valid for its
unit. Novis then enforces nothing for that key. `Core\Config::set` returns `false` for such a
value.
