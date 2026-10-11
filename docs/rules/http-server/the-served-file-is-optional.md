`nvs serve` takes its entry file as an optional argument over a configuration that writes `[[server.mount]]`, and a start that names none serves that mount table whole (`rule:http-server/a-mount-table-expands-at-boot`).

| Start | The table |
|---|---|
| `nvs serve`, mounts written | the written mounts |
| `nvs serve <file>`, mounts written | the written mounts, and `<file>` must be one of their entries |
| `nvs serve <file>`, no mount written | `<file>` at `/`, its own directory as the mount root |
| `nvs serve`, no mount written | refused |

**Nothing runs that neither the command line nor the configuration named.** A start that names no file over a tree that writes no mount is refused, with a line naming `nvs serve <file>` and `[[server.mount]]`; the implicit `public/index.nvs` mount is never reached from `nvs serve`. A `scan` that matched no file is refused the same way, as a table with nothing to answer with. A host of many modules therefore names none of them to start, and removing one module does not stop it starting.

A named file never overrides a written table. One that is not among the mounted entries is refused before a socket exists, because a command told to serve a file and then answering with a different application is the outcome neither reading wants.

**The named file selects no configuration.** One publish holds the host's snapshot, with no `[[app]]` block folded, and one snapshot per entry file, folded from the blocks that match that file (`rule:config/every-matching-app-block-applies-least-specific-first`). A served request runs under the snapshot of the file it runs, and a mount whose entry no block matches runs under the global configuration. The door, and every process-wide key, read the host's. A reload builds the whole publish again, host and every mounted entry, and a row a later expansion adds is folded into the publish standing then, its `[[app]] origin` included. A start with no file and a start with one therefore serve the same configuration.

The file a request runs is the one matched, because an application is its entry file path (`rule:config/an-application-is-its-entry-file-path`): under `dispatch = "path"` that is the file the request resolved to, not the row's `entry`. A spawned script runs under its parent's configuration and never under the blocks of the file it spawns, so a child narrows its parent and never widens it. Matching runs once per entry per publish, never per request: eagerly for every mounted entry when the publish is built, so a block that does not fold for one of them refuses the boot or the reload with `E0601`, and on first use for a file compiled later, which the publish then keeps. A file whose blocks do not fold runs nothing, because the host's snapshot would hand it grants its own blocks narrowed.

**Per entry is what an `[[app]]` block may write; everything else is process-wide.** A request reads `[limits]` and `[limits.hard]`, `[capabilities]`, `[log]`, `mode` and `origin` off its own entry's snapshot. Every other key is read once for the process, or by the door before a mount is selected, from the host's: `[server]`, the mount table, `[[schedule]]`, `[queue]` and `[db.*]` among them. A key a block cannot write is process-wide, because a value read before a mount is selected cannot be per entry. Admission reserves, per request, the largest memory cap any snapshot of the publish reaches, and an entry with no cap leaves none.

Cost: one map lookup and one `Arc` clone per request, after the mount is selected, with no syscall. The door's policy comes from the host's snapshot alone, so alternating mounts never derive it again.

`nvs service install` stores `serve` with no file only where the configuration the argv names writes a mount table that mounts at least one entry on disk, and refuses it otherwise (`E0630`): a server with nothing to serve exits at once, which a service manager reports as a crash loop. A table with no `[server] root` never gets that far, since the configuration does not resolve (`rule:http-server/a-mount-table-expands-at-boot`).

The two refusals of a start are one sentence on standard error and exit status 1, with no span: the command line is what is incomplete, and every line of the configuration is well formed.

**Declined:**
- **Per-entry keys on `[[server.mount]]`.** A mount routes and carries no policy (`rule:http-server/a-mount-carries-no-policy`), and policy keyed on a mount would not reach `nvs run`.
- **The file optional wherever the mount table expands, the implicit mount included.** A bare `nvs serve` would then run a file that neither the command line nor the configuration named, found from a root that is itself a default.
- **Keeping the argument required and letting the installer pick a mounted entry.** The stored argv would name a file nobody chose, and the service would still stop starting when that one module is removed.
- **Folding into one snapshot every `[[app]]` block that matches any mounted entry.** A union is a configuration no one application was written against, and a capability one block grants would reach every mount.
- **The configuration file's own path as the entry a start with no file folds by.** An `[[app]]` block whose `root` covers the configuration's directory would match it, which reads as applying to the host and is not what the block says.
