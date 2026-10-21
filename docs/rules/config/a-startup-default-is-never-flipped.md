Four defaults have a right value that differs between the two modes and cannot be `Runtime`-class:
three directives, because each is read outside any request, before there is one to change it, and
one capability, because a request could never set a grant for itself:

| Directive | Class | `production` | `development` |
|---|---|---|---|
| `[server] dispatch` | `System` | `"entry"` | `"path"` |
| `[server] static` | `System` | `false` | `true` |
| `opcache.settle` | `System` | `"1s"` | `"100ms"` |
| `[capabilities.net] internal`, in server work | capability | `[]` | `["127.0.0.0/8", "::1"]` |

The `internal` row is the default where that key is not written. A CLI run takes the `development`
value in either mode, as it takes the file defaults
(`rule:config/no-configuration-file-is-a-complete-configuration`). A written `internal` list replaces
the row whole, so a production server reaches a loopback service only where a line names it.

**A startup row is chosen from the mode the configuration names, is never re-derived by a runtime
mode flip, and is never flippable.** It is chosen when a configuration is published — at boot, and
again at a reload, because all four rows reload. `Core\Config::set` refuses it
exactly as it refuses any `System` directive, and a runtime mode flip re-derives **only**
the five rows of `rule:config/a-mode-is-five-defaults`. Without that separation a flip would appear to
change `dispatch` for a request that had already been dispatched, and a program that may flip itself
to `development` could open every loopback service to itself.

`opcache.validate` is no longer a row: its default is `mtime` in both modes
(`rule:config/opcache-revalidation-is-system-class`). A startup row may still be `System`, because the
objection is always to the *flip*, never to the *default*: a value chosen by a root-owned mode lets no
request move how the process treats its source. The list stays closed at nine rows across the two
tables, and which table a future default belongs in is decided by its changeability class alone.

**What is on disk.** All four rows. `settle` is chosen where the snapshot's revalidation policy is
read (`nvs_config::cache::Revalidation::from_config`), and `dispatch` and `static` by
`nvs_config::server::switches_for`, which the mount table is built from and which a reload compares
to decide whether a core's table is rebuilt. `internal` is `nvs_config::defaults::internal`, read
through `nvs_config::capability::Authority::address_refused` with the run's started mode, which
`nvs_config::Request::run` reads with the request's flip taken off.

**Declined:**
- **One mode table with no `System` row.** It keeps one property in one sentence, but then
  `[mode] default = "development"` enables neither path dispatch nor static files, and a development
  configuration becomes a file people copy without reading. The cost accepted is that every new
  directive has to say which table it belongs in.
