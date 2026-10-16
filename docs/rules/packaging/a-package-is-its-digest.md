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

**Not on disk.** There is no `nvs add`, `fetch`, `update`, `outdated`, `vendor`, `audit` or `publish`
subcommand, no `package.toml` or `package.lock` reader, and nothing in the tree digests a package
archive.
