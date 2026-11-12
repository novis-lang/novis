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

**A row belongs to one application.** The table is `nvs_sessions`, keyed by `(app, id)`, where
`app` is the path of the innermost `[[app]]` block that matched the request: the identity the memory
cache and the sealed session secret already use. The shared store's key is the prefix, that path,
one `NUL` and the id. So two mounts under two `[[app]]` blocks never open each other's sessions, and
two mounts that should share a login share one block. The configuration generation is not in the key,
so a reload signs nobody out. A deployment with no `[[app]]` block is one application with an empty
path (`rule:concurrency/the-memory-tier-is-one-store-per-process`). Two programs that share one data
folder and write no `[[app]]` block still share sessions, so each needs a block of its own or a
`--data` folder of its own. A connection drops the earlier `nvs_session` table when it opens: its
rows name no application, and no Novis release wrote them.

- **Memory:** one SQLite connection and its statement cache per core, O(cores), and nothing per
  session served. The rows are on disk, O(live sessions), because expired rows are swept
  (`rule:http-server/session-expiry-belongs-to-the-store`). Each store call also builds its key
  from the application's path bytes, once per call.
- **Latency:** one indexed read at `start`, and one update for a changed record or one insert for a
  new session, each on the blocking pool, so a core never waits on the disk
  (`rule:http-server/a-core-is-never-blocked-on-a-syscall`).

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
- **The app folder as the application's identity.** Two entry files in two folders of one site
  would stop sharing a session, and `[[app]]` already names an application.
- **A `[session] scope` key.** It would be a second way to name what an `[[app]]` block names.
