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

A production server never starts it: a host that records with `store` on runs `nvs debug` itself, where
and when its operator chooses. The service's listener, its port and its token are the viewer's
(`rule:testing/the-viewer-shows-the-stream-and-hands-off-every-artifact`).

- **Memory:** the service is its own process, so a server holds nothing for it but the child's handle.

**Declined:**
- **The service inside the server process.** Ingesting would scan directories and write the index on a
  core that serves requests (`rule:testing/the-ingester-runs-whether-or-not-anyone-is-looking`).
- **One service per server.** Two ingesters would write one index.
