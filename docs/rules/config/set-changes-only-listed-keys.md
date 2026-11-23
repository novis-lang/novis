`nvs.toml` gives what a request **starts with**. `nvs_config::settable::SETTABLE` is the one list
of the keys a request may then change for itself with `Core\Config::set`, one row per leaf key, and
each row says how:

| How | What `Core\Config::set` accepts |
|---|---|
| `UpToCeiling` | any valid value, wider or narrower, up to the key's ceiling where it has one |
| `OffOnly` | the value `false` and nothing else; `restore` turns the switch back on |

A key the list does not name returns `false` before any other check, whatever its value. So a key
added to the configuration is the operator's alone until somebody adds its row and argues it in the
row's comment. The list is one table rather than a field on each directive, because *who may set it*
and *what applying a change requires* (`rule:config/reloadability-is-its-own-field`) are two
questions with two homes.

A key is listed only when changing it changes what the request that set it does. A key nothing reads
while a request runs is not listed, because a `true` would change nothing: under `[http]` these are
`http.cors.methods`, `http.cors.headers` and `http.cors.max_age`, which shape only a preflight
answered before any program runs, `http.errors.detail`, and `log.access`, whose line is written from
the file's value once the program has ended. `crates/nvs-config/tests/request.rs` walks every key the
default file writes and checks `set` accepts exactly the listed ones, and
`crates/nvs-cli/tests/request_http_settings.rs` checks each listed key changes what the request does.

The ceiling of a limit is `[limits.hard]`, and a key it does not write is the value the run started
with, so a limit widens only where the operator wrote a ceiling above its start
(`rule:config/no-configuration-file-is-a-complete-configuration`). The ceiling of `mode.default` is
`[mode] ceiling`. Every other `UpToCeiling` key has none.

The list is the application's own code's. Code in a package's namespace sets the `limits.*` rows
alone, and only lower, and restores only what a package set
(`rule:security/package-authority-is-granted-one-line-at-a-time`).

**The compiler reads the same list.** A literal name in the first argument of `set` or `restore`
that no row names is `E0667`, and so is a literal key outside `limits.` in a package's namespace. A
template string whose fixed parts no row can fit is warning `W1037`. A variable, a call or a
concatenation gets no diagnostic, because neither the checker nor the author can know its value.
`get` reads every key and is not checked. *What it spends:* one scan of the list per call site while
compiling, and nothing at run time.

This is what lets the most common run-time set, raising `memory` for one import, work, while
`rule:config/ceilings-are-their-own-directives` keeps one request from becoming every co-resident
request's outage. No runtime set crosses an isolation boundary
(`rule:config/a-runtime-set-is-request-local`), so **Security and request isolation** is not what
widening trades. *What it spends:* one scan of the list's rows per `set`, and no memory.

**Declined:**
- **Narrowing only, for every limit.** The commonest run-time set raises a limit, so code that does it
  would fail at run time with the out-of-memory error or timeout it was trying to prevent.
- **A hardened mode that makes every limit narrowing-only again.** Two configuration semantics to
  implement, test and reason about, for what one ceiling per directive already says.
- **A refuse list, or a class on each directive.** Both make a new key settable unless somebody
  notices: a directive added under a settable block, or missing from the refuse list, is a key every
  request may change. A list of what is allowed fails closed.
