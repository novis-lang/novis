`[limits]` sets the limits that each request starts with, and `[limits.hard]` sets the highest
value that a request may raise each limit to.

The main keys are `memory`, `cpu_time`, `wall_time`, `max_tasks`, `max_output` and
`max_regex_steps`. `cpu_time` is the processor time that the request uses for computing. Time spent
waiting for a database or a socket does not count. `wall_time` is the elapsed time, waiting
included. A program raises its own limit with `Core\Config::set`. A key without an entry in
`[limits.hard]` has no ceiling.

When a request reaches a limit, Novis stops the request. This is not an exception, so `catch` does
not catch it. The function registered with `Core\Fatal::onLimit` runs, `FATAL: …` is written to
standard error, and `nvs run` exits with status `1`. Output that was already written stays.

**In plain words:** `[limits]` is a spending limit that you may raise yourself. `[limits.hard]` is
the limit that the bank sets, and you cannot raise yours above it.

**Good to know:** in this version, `nvs run` enforces `memory` and `cpu_time`. It stores
`wall_time`, `max_tasks` and `max_output`, and it does not stop a program that exceeds them.
