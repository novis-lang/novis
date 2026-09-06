`opcache.validate` — `never`, `mtime` or `hash` — and `opcache.revalidate_freq` are `System`-class:
a request cannot loosen how often, or whether, the process re-checks source files. Letting a request
set `validate = "never"` for itself would be a way to pin a version of the code past a since-shipped
fix, and letting it lower `revalidate_freq` would be a way to force a `stat`/hash storm on a hot
file. Neither is a request-local decision (`rule:config/system-means-a-request-may-not-set-it`).

`validate`'s **startup default** is selected by the run mode — `never` in `production`, `mtime` in
`development` — as one of the startup rows in `rule:config/a-startup-default-is-never-flipped`. That
does not loosen the paragraph above: the row is chosen by root-owned configuration before any request
exists, is never re-derived by a runtime mode flip, and stays unflippable from code. An `[opcache]
validate` written beside `mode = "development"` still wins, because the mode supplies a default and
nothing more.

`revalidate_freq` is deliberately not a mode row: no value of it a developer's machine needs differs
from an operator's, so it keeps its own default under either mode.
