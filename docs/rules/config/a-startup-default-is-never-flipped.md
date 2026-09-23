Three directives have a right value that differs between the two modes and cannot be `Runtime`-class,
because each is read outside any request, before there is one to change it:

| Directive | Class | `production` | `development` |
|---|---|---|---|
| `[server] dispatch` | `Boot` | `"entry"` | `"path"` |
| `[server] static` | `Boot` | `false` | `true` |
| `opcache.settle` | `System` | `"1s"` | `"100ms"` |

**A startup row is chosen from the mode the configuration names, is never re-derived by a runtime
mode flip, and is never flippable.** It is chosen when a configuration is published — at boot, and
again at a reload for a row whose directive reloads, such as `settle`. `Core\Config::set` refuses it
exactly as it refuses any `Boot` or `System` directive, and a runtime mode flip re-derives **only**
the five rows of `rule:config/a-mode-is-five-defaults`. Without that separation a flip would appear to
change `dispatch` for a request that had already been dispatched.

`opcache.validate` is no longer a row: its default is `mtime` in both modes
(`rule:config/opcache-revalidation-is-system-class`). A startup row may still be `System`, because the
objection is always to the *flip*, never to the *default*: a value chosen by a root-owned mode lets no
request move how the process treats its source. The list stays closed at eight rows across the two
tables, and which table a future directive belongs in is decided by its changeability class alone.

**What is on disk.** No `settle` row.
