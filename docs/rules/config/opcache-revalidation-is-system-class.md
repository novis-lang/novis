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

`revalidate_freq` is deliberately not a mode row: no value of it a developer's machine needs differs
from an operator's, so it keeps its own default under either mode.

**What is on disk.** `settle` is read, with its startup row, into `nvs_config::cache::Revalidation`,
but nothing waits on it yet: the check that would still runs inside the resolve.
