Three directives have a right value that differs between the two modes and cannot be `Runtime`-class,
because each is read outside any request, before there is one to change it:

| Directive | Class | `production` | `development` |
|---|---|---|---|
| `[server] dispatch` | `System` | `"entry"` | `"path"` |
| `[server] static` | `System` | `false` | `true` |
| `opcache.settle` | `System` | `"1s"` | `"100ms"` |

**A startup row is chosen from the mode the configuration names, is never re-derived by a runtime
mode flip, and is never flippable.** It is chosen when a configuration is published — at boot, and
again at a reload, because all three directives reload. `Core\Config::set` refuses it
exactly as it refuses any `System` directive, and a runtime mode flip re-derives **only**
the five rows of `rule:config/a-mode-is-five-defaults`. Without that separation a flip would appear to
change `dispatch` for a request that had already been dispatched.

`opcache.validate` is no longer a row: its default is `mtime` in both modes
(`rule:config/opcache-revalidation-is-system-class`). A startup row may still be `System`, because the
objection is always to the *flip*, never to the *default*: a value chosen by a root-owned mode lets no
request move how the process treats its source. The list stays closed at eight rows across the two
tables, and which table a future directive belongs in is decided by its changeability class alone.

**What is on disk.** All three rows. `settle` is chosen where the snapshot's revalidation policy is
read (`nvs_config::cache::Revalidation::from_config`), and `dispatch` and `static` by
`nvs_config::server::switches_for`, which the mount table is built from and which a reload compares
to decide whether a core's table is rebuilt.
