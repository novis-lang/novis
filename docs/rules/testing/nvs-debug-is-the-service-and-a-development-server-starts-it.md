`nvs debug` runs the debug service — the ingester and the viewer — for one data folder, and a
development `nvs serve` with `[debug] store` on starts it as a child process and stops it when the
server exits. It also runs by hand, for a spool that `nvs run`, `nvs test` or a copied-down directory
filled, and `nvs debug --once` reads what is ready, applies the deletion window and exits.

One service per data folder, because the index has one writer
(`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`). The service holds
`debug/service.lock` in the data folder for as long as it runs. A server that finds the lock held uses
the running service and does not stop it on exit; a server that started its own stops it on exit, and
the child also exits within one ingest pass of noticing its parent is gone, so a killed server leaves
no service behind on any platform. Spool files written while no service ran wait for the next one.
The service reads `[debug] index_age` and `index_max` from the configuration of the server that started
it, and from the working directory's when it is run by hand, as every command does.

The server starts the child before it binds a socket, so the child holds no listener. It runs `nvs
debug --parent`, a hidden flag, with the server's `--data` and `--config`, and a pipe as its standard
input. The pipe is how the child knows its parent is gone: the server closes it on exit, and every
platform closes it when the server dies, however it dies. A `--parent` child that finds the lock held
exits quietly, which is how the server uses the running service. The server prints one line saying
the service reads its records into the index.

A production server never starts it: a host that records with `store` on runs `nvs debug` itself, where
and when its operator chooses. The service's listener, its port and its token are the viewer's
(`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).

- **Memory:** the service is its own process, so a server holds nothing for it but the child's handle
  and its pipe. The child spends one thread blocked on its standard input.

**Declined:**
- **The service inside the server process.** Ingesting would scan directories and write the index on a
  core that serves requests (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`).
- **One service per server.** Two ingesters would write one index.
- **A `--parent <pid>` the child polls.** It needs a liveness check per platform, and a process id can
  be reused by another process once the server is gone. A pipe needs neither.
- **The server checks the lock before it starts the child.** The lock can change hands between the
  check and the child's start, so the child has to handle a held lock anyway.
