`[mode]` sets whether an application starts in `production` mode or in `development` mode.

The block has two keys. `default` is the mode that an application starts in. `ceiling` is the
highest mode that code may select, and `development` is higher than `production`. When `ceiling`
is not set, it is the same as `default`. The mode gives the defaults of the logger and of the HTTP
error pages.

A program reads the mode with `Core\Config::get("mode.default")`.
`Core\Config::set("mode.default", "development")` changes the mode for the current request. It
returns `true` when `ceiling` allows `development`, and `false` when it does not. Only `nvs.toml`
sets `ceiling`. The compiler checks a name written as text in the call, so
`Core\Config::set("mode.ceiling", …)` does not compile.

**Good to know:** the name to read is `mode.default`. `Core\Config::get("mode")` returns `null`,
and `Core\Config::set("mode", …)` does not compile.
