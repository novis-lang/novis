A package's **identity is the BLAKE3 digest of its archive** — the hash the artifact cache already
uses, so the toolchain carries no second one. Two archives with one digest are the same package; two
with different digests are different packages regardless of the name either carries. A name is only
a way to find a digest, and `rule:packaging/the-lockfile-holds-every-digest` is what makes that
identity verifiable on every build.

The archive holds Novis source and data files only — no compiled artifacts, no shared objects, no
binaries — with exactly one exception: an **extension package**, whose payload is a `.nvsx` wasm
component sandboxed by the extension system and carrying the qualifier declarations an extension
manifest must (`rule:security/extension-manifest-only-tightens`). An extension package may also carry Novis source
beside its component, a builder composing calls into it; that source is resolved exactly as a source
package's is, while the `.nvsx` alone is what the `[[extension]]` pin governs. Both kinds are named,
resolved, pinned, granted, logged and vendored identically. There is no third kind, and a native
subsystem of the standard library is never a package.

Compiled artifacts are never published: a compiled unit is unreviewable, and source distribution is
what makes "the digest is the identity" a statement about code a human could read.

**The archive is deterministic**, so the same files give the same digest on every platform and the
registry, building from the commit, gets the bytes the publisher got
(`rule:packaging/the-registry-builds-the-archive-from-git`):

- It is an **uncompressed POSIX `ustar` tar**, written through the `tar` crate
  (`rule:packaging/somebody-elses-specification-is-a-crate`). A transport may compress it; the digest
  is over the uncompressed bytes.
- It holds the files `rule:packaging/a-package-carries-only-its-source-and-what-it-lists` names,
  **sorted by the bytes of their paths**, each a regular file. A path over 100 bytes is split
  between the header's `prefix` and `name` fields at a `/`, as `ustar` says, and a path with no such
  split is refused. There is no directory entry, link, special file or extended header.
- **Every field not taken from the file is fixed:** mode `0644`, owner and group `0` with empty
  names, time `0`, and the device fields left zero bytes. A file's time, owner, mode and the order a file system
  lists it in never reach the bytes.
- It ends with the two zero blocks `ustar` requires, and nothing after them.

`nvs pkg pack` writes it and prints its digest (`crates/nvs-cli/src/package/pack.rs`), and
`crates/nvs-cli/tests/package_client.rs` pins one fixture's digest, so a change to the format is a
deliberate change to every package's identity.

**The registry's files have one schema**, which the client defines
(`crates/nvs-cli/src/package/index.rs`) and the registry's service writes. Each file carries
`"schema": 1`, and a client refuses a schema it does not read, and every key it does not know. The
registry's URL serves `registry.json`, whose `hosts` are the file hosts in the order a client tries
them (`rule:packaging/registry-hosting-is-configuration`); with no `hosts`, the files are under the
registry's URL. Under each file host:

- **`index/<vendor>/<name>.json`**, named by the package's slug, holds `name` (the namespace),
  `owners`, and `versions`. Each version has `version`, `digest` (64 lowercase hex digits),
  `state`, `dependencies` (each namespace to its minimum), `capabilities` (`required` and
  `optional`), and the `repository` and `commit` the registry built it from.
- **`state`** is `"released"`, `"unlisted"`, `{"pending": {"selectable_at": <Unix seconds>}}`,
  `{"retracted": {"reason": ..., "fixed_in": <version or null>}}` or
  `{"withheld": {"reason": ...}}`.
- **`archive/<digest>.tar`** is the archive. A client tries the next file host when a host does not
  answer or when the bytes it serves do not have the digest.

```json
{"schema": 1, "hosts": ["https://files.example.com"]}
```

**Not on disk.** No command fetches a package yet: the index, the lock and the archive's digest
check exist (`crates/nvs-cli/src/package/`), and `nvs pkg fetch` does not.
