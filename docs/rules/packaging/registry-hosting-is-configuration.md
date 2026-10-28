The registry's hosting is configuration. The same `apps/registry/` runs on a developer's machine with
every file in a local directory, and in production with its public files on a CDN; the difference is
one settings file the deployment writes, never a code change.

**The settings file is `apps/registry/hosting.json`,** read when the service starts. The committed copy
is the local one: every store is a directory under the application's data folder, so the whole service
is developed and tested on one machine. A deployment replaces the file. A secret in it, such as a
store's credential or the backup key, is the path of a file, read through `Core\Secret` so it never
prints, and a path that does not exist stops the start and names the path. The file names:

- **the public stores**, where the static files are written: the root file, the index files, the
  archives, the log tiles and the checkpoint. A store is `dir`, a directory, or `http`, where each file
  is a `PUT` to a base URL with one credential header, which is Bunny Storage's API and any WebDAV
  server's. Every public store receives every file.
- **the file hosts**, the `https:` URLs clients read the public files from, in order. The service
  writes them into the root file `registry.json` at the registry's URL, the one file a client reads
  from the registry's own host (`rule:packaging/a-vendor-resolves-from-one-registry`). The root file
  is signed like an index file, and the client keeps the last one it read, so it still finds the file
  hosts when the registry's host does not answer.
- **the backup store**, `dir` or `http`, which is never a public store.
- **the download-log source**, where the file hosts' access lines come from, when the service does not
  serve the files itself.

**A publish is durable before it is visible.** The service writes the archive, then the log tiles,
then the signed index files, and the checkpoint last, to every public store, and answers the publish
only after every store has confirmed every write. A store that fails fails the publish, which the
publisher retries, and nothing a client reads has moved. Why: a client refuses a checkpoint that does
not extend the newest one it saw (`rule:packaging/the-registry-keeps-an-append-only-log`), so a
checkpoint a client saw and the service then lost would make the next one a fork. The CDN's store,
off the host, is that durable copy.

**Files are cached by whether they change.** An archive and a full log tile never change, and the
deploy guide serves them with a cache lifetime of a year. The root file, the index files, a partial
tile and the checkpoint change, and are served with one of 60 seconds. The service never purges the
CDN, so it holds no CDN account key.

**The database is copied to the backup store** after each publish and at an interval the settings set,
five minutes by default, and not when nothing changed since the last copy, which a SHA-256 of the
copy tells. The copy is every row of every table as JSON lines, read in one transaction, sealed with
`Core\Crypto::seal` under XChaCha20Poly1305 and a key the settings name, so the backup store never
reads it, and it replaces the one before. It is not a `VACUUM INTO`, because Core's SQLite driver
refuses that statement: it writes a file no `fs` grant was checked for. The log is the record of
every release, so a lost interval loses at most the rows the log does not hold, such as a token
created in it, which its owner creates again.

**A service that starts with no database restores itself.** When the backup store has a copy, the
service opens it, takes the log from a public store, runs `nvs registry log verify`, rebuilds every
release's state from the log, and starts. A new host with the same settings comes up restored, with no
step by hand. The deploy guide stops the old host first, because two services writing one log sign
two checkpoints of one size.

**Downloads are counted from the file hosts' access lines.** When the settings name a download-log
source, a daily job reads the previous day's lines, applies the counting rule of
`rule:packaging/search-sorts-by-the-readers-choice` in memory, and keeps no line. When the service
serves the files itself, as it does locally, it counts as it serves them, through the same function.
Each CDN writes its own line format, so each source is one parser; Bunny's is the first.

**One host runs each service as its own account.** The deploy guide runs the registry and the
telemetry collector as two operating-system accounts in two service units, so the registry's database
and log key are readable by the registry's account alone. The log key is also kept offline, encrypted,
and outside every store, because a lost key cannot sign the next checkpoint.

**Memory:** a publish holds one archive while it writes it to each store, and the repository size
limit bounds it. A database copy is a temporary file the size of the database, deleted once it is
sent. The daily count holds one day's access lines, twice, and their salted hashes while it runs,
and keeps none of them after it.

**Declined:**
- **The service serving every file in production.** An outage of one host would stop every
  `nvs pkg add` and every fresh build. crates.io moved its downloads off its API servers for this
  reason.
- **A counting redirect on the service.** Every download would still need the service to be up.
- **Purging the CDN on publish.** A purge needs an account-wide key on the server, and a 60-second
  lifetime needs none.
- **Replicating the log asynchronously.** A signed checkpoint lost in the lag becomes a fork for every
  client that saw it.
- **A standby that takes over by itself.** Two writers sign two different checkpoints of one size, a
  fork by construction, while a write outage only delays publishers.
- **A provider's SDK or name in the code.** `dir` and `http` cover a local disk, Bunny Storage and any
  WebDAV server, and the CDN is replaced by editing the settings. The access-line parser is the one
  place a provider's format is code.
- **Hosting settings in `nvs.toml`.** It holds the runtime's directives and refuses a name it does not
  know, and hosting is the application's own.

**Not on disk.** `apps/registry/hosting.json` and `Site::load` read the stores, the file hosts, the
backup store and key, and the download-log source, and stop on a file a setting names that does not
exist. `Stores` sends each publish's files from the work folder to every store, one rank at a time
across the stores and the checkpoint last, and a publish of the same commit sends what a failed
one left. `jobs/daily.nvs` signs and sends every index file, `Backup` copies the database after a
publish and from `jobs/backup.nvs`, and `Restore` runs from `Schema::open` on a database with no
tables. `jobs/daily.nvs` also counts the day before from Bunny's access lines through
`Downloads::countLog`, with a salt and a set of hashes held only in memory, and marks the day in
`downloads_counted` so a second run counts nothing.
