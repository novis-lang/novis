`nvs serve` takes its entry file as an optional argument over a configuration that writes `[[server.mount]]`, and a start that names none serves that mount table whole (`rule:http-server/a-mount-table-expands-at-boot`).

| Start | The table |
|---|---|
| `nvs serve`, mounts written | the written mounts |
| `nvs serve <file>`, mounts written | the written mounts, and `<file>` must be one of their entries |
| `nvs serve <file>`, no mount written | `<file>` at `/`, its own directory as the mount root |
| `nvs serve`, no mount written | refused |

**Nothing runs that neither the command line nor the configuration named.** A start that names no file over a tree that writes no mount is refused, with a line naming `nvs serve <file>` and `[[server.mount]]`; the implicit `public/index.nvs` mount is never reached from `nvs serve`. A `scan` that matched no file is refused the same way, as a table with nothing to answer with. A host of many modules therefore names none of them to start, and removing one module does not stop it starting.

A named file never overrides a written table. One that is not among the mounted entries is refused before a socket exists, because a command told to serve a file and then answering with a different application is the outcome neither reading wants.

**The named file selects no configuration.** One publish holds the host's snapshot, with no `[[app]]` block folded, and one snapshot per entry file, folded from the blocks that match that file (`rule:config/every-matching-app-block-applies-least-specific-first`). A served request runs under the snapshot of the file it runs, and a mount whose entry no block matches runs under the global configuration. The door, and every process-wide key, read the host's. A reload builds the whole publish again, host and every mounted entry, and a row a later expansion adds is folded into the publish standing then, its `[[app]] origin` included.

`nvs service install` stores `serve` with no file only where the configuration the argv names writes a mount table that mounts at least one entry on disk, and refuses it otherwise (`E0630`): a server with nothing to serve exits at once, which a service manager reports as a crash loop. A table with no `[server] root` never gets that far, since the configuration does not resolve (`rule:http-server/a-mount-table-expands-at-boot`).
