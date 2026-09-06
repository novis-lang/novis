Three directives have a right value that differs between the two modes and cannot be `Runtime`-class,
because each is read before there is any request to change it:

| Directive | Class | `production` | `development` |
|---|---|---|---|
| `[server] dispatch` | `Boot` | `"entry"` | `"path"` |
| `[server] static` | `Boot` | `false` | `true` |
| `opcache.validate` | `System` | `never` | `mtime` |

**A startup row is fixed at boot, is never re-derived, and is never flippable.** `Core\Config::set`
refuses it exactly as it refuses any `Boot` or `System` directive, and a runtime mode flip re-derives
**only** the five rows of `rule:config/a-mode-is-five-defaults`. Without that separation a flip would
appear to change `dispatch` for a request that had already been dispatched.

The objection to a flippable `opcache.validate` was always about the *flip*, never the *default*: a
request that could set `validate = "never"` for itself would pin a version of the code past a shipped
fix, and a startup value chosen by a root-owned mode does none of that. The list stays closed at eight
rows across the two tables, and which table a future directive belongs in is decided by its
changeability class alone.
