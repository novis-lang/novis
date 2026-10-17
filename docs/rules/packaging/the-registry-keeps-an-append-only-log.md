The registry keeps an **append-only transparency log** in the C2SP formats — tlog-tiles, with
checkpoints as signed notes under an Ed25519 key — and a client verifies an archive's inclusion and the
log's consistency with the newest checkpoint it saw before it uses the archive.

**What the log records** is a closed list of entry kinds, one per event the registry acts on:

- a publish: name, version, digest, repository and commit;
- an ownership change;
- a token created or revoked;
- a repository binding changed;
- an unlist and its reversal;
- a retraction, and an advisory;
- a withhold;
- a release confirmed before its seven days end;
- a stable API confirmed.

Each entry is canonical JSON with a schema version, and the log is the only record of order.

**The client keeps one checkpoint per registry** in the data folder, under the ownership check the
artifact cache uses (`rule:config/ownership-is-the-trust-boundary`). A new checkpoint must come with a
consistency proof from the stored one. An archive absent from the log is refused. An inconsistent
checkpoint is a hard error that names the registry and says the log was forked, never a network error,
and nothing is fetched. `nvs pkg audit` also compares the checkpoint with a second published copy
of it.

A registry that serves one client different bytes than another must therefore fork the log, which the
next client that checks consistency detects, or sign a statement that contradicts one it already made.
Compromising the registry stops being silent, and the lockfile
(`rule:packaging/the-lockfile-holds-every-digest`) then holds the digest so a client need not trust the
registry twice. The log is also why a registry, and not git URLs alone, is the transitive source:
retraction, the advisory feed and `nvs pkg audit` need an authority that cannot quietly rewrite
what it said.

**Outside witnesses can co-sign checkpoints later with no format change**, and none are required now.
The C2SP formats are what make that true, and they are what other transparency logs already speak.

The byte work is `nvs registry log append` and `nvs registry log verify`, the commands the service calls
(`rule:packaging/the-registry-is-a-novis-program`).

**Memory:** a stored checkpoint is a few hundred bytes per registry. A fetch holds one inclusion proof
per archive and one consistency proof per checkpoint while it runs.

**Declined:**
- **A log format of the registry's own.** It would work, and it would shut out every witness and
  verifier that already reads C2SP.
- **Witnesses required from the start.** The format leaves room for them; requiring them now would
  make the registry depend on parties that do not exist yet.

**Not on disk.** There is no registry and no log.
