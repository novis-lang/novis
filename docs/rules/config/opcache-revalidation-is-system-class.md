`opcache.validate` — `mtime` or `hash` — `opcache.revalidate_freq` and `opcache.settle` are
`System`-class: a request cannot loosen how often, or how, the process re-checks source files, or how
long a changed program must be quiet before it is compiled. A request that could lower
`revalidate_freq` or `settle` could force a `stat` storm or a compile per keystroke of somebody
else's deploy, and one that could raise either could hold a shipped fix back. Neither is a
request-local decision (`rule:config/system-means-a-request-may-not-set-it`).

`mtime` is `validate`'s default in both modes, and `hash` is the stricter value, for a file system
whose stamps cannot be trusted. **There is no `never`**: a configuration that writes it, or the
boolean `false` that meant it, does not load, and the error names `mtime` and `hash`. A value that
pins the code a process started with is the outage
`rule:config/an-edit-reaches-the-next-request-without-a-restart` exists to end.

`settle`'s **startup default** is selected by the run mode — `"1s"` in `production`, `"100ms"` in
`development` — as one of the startup rows in `rule:config/a-startup-default-is-never-flipped`. The
row is chosen by root-owned configuration before any request exists, is never re-derived by a runtime
mode flip, and stays unflippable from code. A `settle` written beside `mode = "development"` still
wins, because the mode supplies a default and nothing more.

`opcache.incremental` is `System`-class and a startup row too: `true` in `development`, `false`
in `production`. With it `true`, a server keeps each loaded program's checked files and function
code between compiles, so a reload after a body edit checks one file and compiles one function
(`rule:config/an-edit-reaches-the-next-request-without-a-restart`). With it `false`, both are
freed when a compile ends. A request that could turn it on would make the server hold memory no
operator chose, and one that could turn it off would slow every later reload of a developer's
program. A written value beside either mode wins, as `settle`'s does.

`revalidate_freq` is deliberately not a mode row: no value of it a developer's machine needs differs
from an operator's, so it keeps its own default under either mode.

**A server that relies on `nvs reload` alone sets a long interval**, such as
`revalidate_freq = "1h"`, in place of `never`. A developer or a coding agent then makes each batch
of edits live with `nvs reload` (`rule:config/a-reload-command-checks-now-and-waits`), and a
forgotten reload still goes live within the hour.

**Declined:**
- **Keeping `never` for operators who want it.** It is the outage a restart-free deploy removes, and
  the cost it saved is one background `stat` per file per window. A value kept for compatibility
  would keep that outage.
- **One `incremental` default for both modes.** A developer's machine reloads after every save and
  wants each reload small. A server is updated a few times a day and wants its memory back after
  each compile. No one value serves both.
- **A mode-five row for `incremental`.** A runtime mode flip would then make a running server keep
  or free memory, which only the person who runs it may decide.