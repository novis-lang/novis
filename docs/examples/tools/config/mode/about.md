`[mode]` sets whether an application starts in `production` mode or in `development` mode.

The block has two keys. `default` is the mode that an application starts in. `ceiling` is the most
permissive mode that code may select. When `ceiling` is not set, it is the same as `default`. The
mode selects the defaults for the logger and for the HTTP error pages.

A program reads the mode with `Core\Config::get("mode.default")`.
`Core\Config::set("mode.default", "development")` changes the mode for the current request. It
returns `true` when `ceiling` allows `development`, and `false` when it does not. Only the file
sets `ceiling`, so `set` always returns `false` for `mode.ceiling`.

**Good to know:** the name to read is `mode.default`. `Core\Config::get("mode")` returns `null`,
and `Core\Config::set("mode", …)` returns `false`.
