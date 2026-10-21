A tree that writes no `[session] backend` gets the `db` store: `session.sqlite` in Novis's own data
folder (`rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`), in WAL mode. A first
program that calls `Core\Session::start` needs no configuration. Every server process on the machine
that shares the data folder reads the same file, so a request another process handles finds the
session, and a restart keeps it. That is the reason the memory tier is refused
(`rule:http-server/session-backend-is-shared-or-db-and-memory-is-refused-at-boot`), and the file
meets it. `shared` stays the store for several machines.

A process with no usable data folder, one that could not be created or failed
`rule:config/ownership-is-the-trust-boundary`'s check, has no default store. There `start` throws a
`RuntimeError` naming `[session] backend = "shared"`, and every other member throws until `start`
has run (`rule:core-classes/session-is-started-explicitly`).

The file is Novis's own: the operator chose the folder, the name is fixed, and no program-supplied
path reaches it. So no capability is asked, as none is for the compiled-code cache beside it. Each
operation is one short statement in its own transaction, so two requests of one session never wait
for each other (`rule:http-server/a-session-is-loaded-once-and-written-whole`). `busy_timeout`
covers the moment two writers meet, and that wait is one statement long.

- **Memory:** one SQLite connection and its statement cache per core, O(cores), and nothing per
  session served. The rows are on disk, O(live sessions), because expired rows are swept
  (`rule:http-server/session-expiry-belongs-to-the-store`).
- **Latency:** one indexed read at `start` and one upsert for a changed record, each on the blocking
  pool, so a core never waits on the disk (`rule:http-server/a-core-is-never-blocked-on-a-syscall`).

**Declined:**
- **No store at all where nothing is written.** It was safe and it was useless: a first program that
  wanted a session had to choose a store and run a shared cache before its first page worked. A file
  in the data folder is coherent across the processes of one machine, which is what the memory
  tier's refusal asks for.
- **A table in a `[db.<name>]` for `db`.** It needs a schema in the operator's database and a grant
  to write it. The data folder's file needs neither, and a deployment with several machines has
  `shared`.
- **A session lock, as PHP's file handler takes.** One slow request would hold every other request
  of that session until it ends (`rule:http-server/a-session-is-loaded-once-and-written-whole`).
