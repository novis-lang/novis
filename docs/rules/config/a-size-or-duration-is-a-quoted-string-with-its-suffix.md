A size or a duration is written as a **quoted string carrying its suffix** — `memory = "128M"`,
`cpu_time = "5s"` — never as a bare integer of implied units. The suffix is what makes the file readable
at a glance, and the parser for it has to exist anyway for `Core\Config::set("memory", "512M")`
(`rule:config/one-parser-for-the-boot-path-and-config-set`). A count, such as `max_tasks = 64`, is an
ordinary integer.

"No ceiling" is a boolean, and TOML has one: `[limits.hard] memory = false` removes the ceiling, and
`memory = "2G"` enforces it. There is no magic word `off`.

Every size suffix names the same binary multiple and every duration suffix the same span of
nanoseconds wherever the value is spelled, and one limit has one unit in every block that carries it —
`[limits]`, `[limits.hard]`, `[app.limits]` and `[app.limits.hard]` agree on what `memory` measures.
