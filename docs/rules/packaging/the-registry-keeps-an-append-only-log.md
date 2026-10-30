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

**Every index file is signed, and its age is bounded.** The root file and each package's index file are
signed notes under the log key, written by `nvs registry index sign`. The note's text is the file's
JSON, which carries the time it was signed and the size and root hash of the checkpoint it was signed
with. The service signs every index file again each day. The client refuses an index file whose
signature does not verify, whose checkpoint is not consistent with the stored one, or whose time is
more than an hour ahead of the client's clock, and tries the next file host
(`rule:packaging/registry-hosting-is-configuration`). A validly signed checkpoint that is not
consistent stays the fork error, and no other host is tried. `nvs pkg add`, `nvs pkg update` and
`nvs pkg outdated` refuse an index file signed more than seven days ago and name the registry and the
time. A build from the lock and `nvs pkg fetch --locked` select nothing, need only digests and
inclusion, and work through any outage.

The index is what tells a client which version is released, pending, retracted or withheld. A host
that could change it could skip the seven-day window or show a withheld version as released, and one
that could serve an old copy could hide a fix for as long as it liked. The signature stops the first,
the age bound limits the second to seven days, and the lock keeps every build working while the
service is down.

**The registry's verifier key is pinned, never learned.** The default registry's key is a constant in
`nvs`. A vendor that `package.toml` maps to another registry names that registry's key beside its URL,
in the C2SP note verifier-key form (`rule:packaging/a-vendor-resolves-from-one-registry`).

A registry that serves one client different bytes than another must therefore fork the log, which the
next client that checks consistency detects, or sign a statement that contradicts one it already made.
Compromising the registry stops being silent, and the lockfile
(`rule:packaging/the-lockfile-holds-every-digest`) then holds the digest so a client need not trust the
registry twice. The log is also why a registry, and not git URLs alone, is the transitive source:
retraction, the advisory feed and `nvs pkg audit` need an authority that cannot quietly rewrite
what it said.

**Outside witnesses can co-sign checkpoints later with no format change**, and none are required now.
The C2SP formats are what make that true, and they are what other transparency logs already speak.

The byte work is `nvs registry log append`, `nvs registry log verify` and `nvs registry index sign`,
the commands the service calls (`rule:packaging/the-registry-is-a-novis-program`). The client's
checks are `crates/nvs-cli/src/package/proof.rs` for the log and
`crates/nvs-cli/src/package/index.rs` for the signed files.

**Memory:** a stored checkpoint and the last root file are a few hundred bytes per registry. A fetch
holds one inclusion proof per archive and one consistency proof per checkpoint while it runs. The
daily signing writes every index file again, one small file per package per store per day.

**Declined:**
- **A log format of the registry's own.** It would work, and it would shut out every witness and
  verifier that already reads C2SP.
- **Witnesses required from the start.** The format leaves room for them; requiring them now would
  make the registry depend on parties that do not exist yet.
- **Unsigned index files.** Only the archive would be checked, and whoever serves the files would
  decide every version's state.
- **A snapshot file listing every index file's hash.** It stops the rollback of one file, but every
  client downloads a file that grows with the number of packages. A daily signature on each file
  bounds a rollback with nothing extra to download.
- **Refusing every command on an old index.** A long outage would then break every build, while the
  lock already holds what a build needs.
- **A time line of the registry's own in the checkpoint.** The checkpoint stays C2SP's, and the time
  is in the index file, whose format is the registry's.
- **The key the registry serves.** Whoever controls the files would then also choose the key.

**The formats are written in this tree**, the exception
`rule:packaging/somebody-elses-specification-is-a-crate` allows when it is argued. The tiles, the
hashing and both proofs are `crates/nvs-cli/src/package/log.rs`, on the `sha2` the tree already
links, with no new crate. The only crates for the formats, `tlog_tiles` and `signed_note`, are not
first-class: each is at 0.2 with one maintainer. They also sit on the older `sha2` 0.10 line. And
`tlog_tiles` pulls in `url` and its ICU stack, and `signed_note` a second `ed25519-dalek` beside the
one already linked. That is some twenty crates for about five hundred lines that follow Go's
`golang.org/x/mod/sumdb/tlog`. The code is held to the vectors of transparency-dev's `merkle`
module and of C2SP's and Go's tile paths, vendored under `crates/nvs-cli/tests/fixtures/tlog/`.
The cost is that this tree owns the code for as long as the log exists. The risk is bounded: a
proof's arithmetic is fixed by RFC 6962, and a wrong proof fails against every other verifier.

The registry service under `apps/registry/` runs these commands. `nvs pkg audit` compares the second
copy of the checkpoint a registry's root file names with the log the file host serves, and stops on
a disagreement (`crates/nvs-cli/src/package/audit.rs`). The client reads the default registry once its
verifier key is in the binary as `DEFAULT_KEY` (`crates/nvs-cli/src/package/manifest.rs`), and reads
nothing from it while that is `None`.
