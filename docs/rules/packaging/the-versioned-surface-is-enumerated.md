A version number is meaningless unless the thing it describes is listed. Novis versions **exactly
this**: what parses and what it means; every `Core` member's signature and observable behaviour;
`nvs.toml` keys, their defaults and their changeability classes; the `nvs` CLI's subcommands, flags,
exit codes and machine-readable output; diagnostic *identity* — a code or name an editor, CI or a
suppression comment can key on; the extension ABI a third-party `.nvsx` is compiled against; any
format that outlives the process and can be read back, `serialize()` output above all; and the
supported target platforms and database server versions.

And **explicitly does not version**:

- **The Rust APIs of the `nvs-*` crates.** They share the workspace number as a build convenience
  and carry no stability promise whatsoever; a Rust program depending on `nvs-hir` depends on an
  internal.
- **Anything regenerable from source.** The on-disk artifact cache is self-describing and a format
  change is a cache miss by construction — never a break, never a version event. The same holds for
  a portable bundle, which carries the host that reads it (`rule:programs/bundle-trust-domain`).
- **Exact diagnostic prose, timing figures, or the byte layout of anything not listed above.**
  Wording improves between patches.
- **The minimum supported Rust version.** Users receive a binary; MSRV is a build-from-source fact,
  announced in release notes and never a break.

The contract is not in force until `rule:packaging/the-version-contract-starts-at-0-1-0` throws its
switch.
