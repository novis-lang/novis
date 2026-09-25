---
milestone: post-parity
---
# Loop goal 10 — `Core\Program::id()`, the program's own fingerprint

Give userland the identity the runtime already stakes correctness on: one member,
`Core\Program::id(): string`, answering a stable hex fingerprint of *exactly this build of this
application* — the same on every process serving the same sources on the same runtime, different the
moment any source file, the runtime build or the extension set changes. It is the deploy hash every web
stack eventually grows by hand (a git SHA baked in at build, an env var someone forgets), derived here by
the runtime so it is always present and always honest.

This goal is one member and one computed value: the smallest goal on the chain. Its floor is goal `temp-sweep`'s
whole list, which is the parity program plus the temp sweep.

## Why here

Post-parity: `Core\Program::id()` — the program fingerprint the freeze/thaw design needs and
userland gets.

## What the member answers, in the two registers the card carries

**In simple words** (the registry card's short description): *The program's fingerprint: one short hex
text that names exactly this build of this application. Every process running the same sources on the
same runtime answers the same text, and any change — a source file, the runtime, an extension — makes it
a different text.*

**In detailed words** (the reference doc's body): `Core\Program::id()` answers the lowercase-hex spelling
of `BLAKE3(unit content hashes in program order ‖ env_hash)` — the per-unit source digests
`rule:packaging/an-artifact-is-one-immutable-content-addressed-file`'s artifact cache already computes, combined
in program order, then the environment hash covering the target triple, the CPU feature bitset, the
compiler build and the loaded extension set
(`rule:config/the-config-is-an-immutable-snapshot`). It is computed once when the program is
resolved, and again when a hot reload swaps code in
(`rule:config/an-edit-reaches-the-next-request-without-a-restart`) — never per call. Two properties are the
contract: **deterministic** — every worker process, restart and machine serving the same sources on the
same runtime answers the same id, so it is usable as a cache key; and **complete** — any change to any
source file in the program graph, to the runtime build or to the extension set changes it, so nothing
keyed on it survives a deploy. Its uses: asset cache-busting query strings, cache-key namespacing so a
new deploy never reads an old deploy's entries, a release tag in logs and error reports, an
"app updated, please reload" comparison header, and deploy-scoped validity for values the application
serializes. What publishing it reveals is deploy *timing* — an observer can tell that a deployment
changed, never what changed; a deployment that treats timing as sensitive truncates or keys the id before
publishing it.

## Stage 0 — the catch-up

Nothing. No fixture predates the rule; the ADR amendment lands inside this goal.

## Stage 1 — the floor

Goal `temp-sweep`'s whole acceptance list — the parity program and the temp sweep, never traded.

## Stage 2 — the keystone: the value, computed once and threaded

1. **The combine.** A `program_id` beside `content_hash` and `env_hash` in
   `crates/nvs-config/src/cache.rs:120` / `:105`, reusing both — BLAKE3 over the unit content hashes in
   program order, then the `env_hash`. No second hashing of any source: the per-unit digests are the ones
   the artifact cache already computed.
2. **The threading.** `resolve_program` (`crates/nvs-hir/src/requires.rs:182`) is where the unit set
   becomes final; the caller that already holds the `env_hash` owns the combine, and the result is stored
   in the per-program state the runtime reads (`crates/nvs-runtime/src/ctx/wiring.rs` is the neighborhood — the
   session picks the exact seam). The hot-reload swap recomputes it with the units it swaps in.

## Stage 3 — the member, its card, its proofs

1. **`Core\Program::id()`** — the `Core` member shape's full set of edits, in a new
   `crates/nvs-stdlib/src/program.rs`, with the registry card
   (`crates/nvs-stdlib/src/registry.rs`) carrying the two descriptions above per
   `rule:core-api/reference-card`, and the
   reference doc at `docs/reference/core/Program.md`. Zero arguments, pure, no capability gate — it reads
   program state and touches no I/O.
2. **The conformance case** — `tests/conformance/core/` — asserting in-language what frozen output can
   assert about a value that differs per build: 64 lowercase hex characters, and two calls within one
   run agree.
3. **`examples/program-id.nvs`** — the same two properties as a runnable fixture with deterministic
   output, plus the named unit tests of the TOML: stability for identical inputs, change on one unit's
   content, change on the env hash.

## Standing decisions

- **The formula is fixed and is not re-derived**: `BLAKE3(unit content hashes in program order ‖
  env_hash)`, spelled as all 32 bytes in lowercase hex — 64 characters. Userland truncates if it wants
  fewer; the runtime never does.
- **This goal may change `rule:programs/no-runtime-autoload` through one
  record whose `changes:` block names it, and open no other number**: a section giving `Core\Program` its first and only runtime
  member, carrying the formula and the reason it *cannot* be a compile-time-folded constant — folding the
  id into any unit as a literal would change that unit's bytes, hence its content hash, hence the id it
  just folded. Circularity, not preference.
- **Plain `string`, never `secret`** (`rule:security/secret-qualifier`):
  every use of the id — URLs, headers, logs — is an echo, which `secret` refuses by design. The
  fingerprinting caveat (deploy timing, nothing else) is recorded in the registry card, not enforced.
- **Computed at program resolution and at the hot-reload swap, never per call and never lazily** — a
  first-call compute would put a hash of every unit digest on one unlucky request's path. What this
  spends: 32 bytes per program and one BLAKE3 combine per resolution, per
  `rule:programs/memory-priority`'s ledger.
- **Ambiguity about the seam resolves toward the existing hashes' home** — `nvs-config/src/cache.rs`
  owns the combine as it owns its two inputs; decided-and-recorded in that module's doc comment, never
  `BLOCKED`.
