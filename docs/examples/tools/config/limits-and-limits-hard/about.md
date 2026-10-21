`[limits]` sets the limits that each request starts with. `[limits.hard]` sets the highest value
that a request may raise each limit to, with `Core\Config::set`.

The main keys are `memory`, `cpu_time`, `wall_time`, `max_tasks`, `max_output` and
`max_regex_steps`. `cpu_time` counts only computing time. `wall_time` counts waiting too. When
`[limits.hard]` does not set a key, a program can lower that limit but cannot raise it. A program
you start from the command line may raise its `memory` limit to any value, unless `[limits.hard]`
sets `memory`.

By default, `memory` is `128M` and a request has `30s` of `cpu_time`. `nvs run` and `nvs test`
have no `cpu_time` limit unless you set one. `false` removes a limit.

When a request reaches a limit, Novis stops it. This is not an exception, so `catch` does not catch
it. The function given to `Core\Fatal::onLimit` runs, and `nvs run` exits with status `1`.

**Good to know:** in this version, `nvs run` enforces `memory` and `cpu_time`. It does not stop a
program that exceeds `wall_time`, `max_tasks` or `max_output`.
