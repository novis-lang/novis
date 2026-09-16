# Handoff

## State

**Goal `unowned-closures`, stage 3 closed.** Two gaps struck, and the front end now has **one escape
grammar**. It moved down to `crates/nvs-syntax/src/string_lit.rs` — out of `nvs-types`, which depends
on `nvs-hir` and so sits *above* name resolution — and `nvs_types::string_lit` is a `pub use` of it, so
no call site moved. `cook_quoted` (`crates/nvs-hir/src/requires.rs:1276`) is now a spelling filter in
front of `cook_string_literal`: a `require` path, an `autoload` prefix, a root and a `discover` glob all
decode octal, hex and `\u{...}` exactly as an ordinary string literal does, where before they took a
practical subset and left the backslash verbatim.

`rule:packaging/autoload-probes-fold-into-the-cache-key`'s probe trace is recorded rather than dropped.
`AutoloadMap::resolve_recording` (`crates/nvs-hir/src/autoload.rs:315`) keeps every probed path, misses
included, in the `ProbeTrace` (`crates/nvs-hir/src/autoload.rs:163`) the map already carries back, and
the graph walk is its only caller — so `nvs check --autoload-map` or an editor resolving a name cannot
lengthen a trace a cache key is computed from. **Nothing reads it yet**: the fragment says so as *Half
on disk*, and `requires.rs`'s own known gaps name the cache-side half that is left.

Nothing is blocked, and **`python tools/rules.py --check` is green tree-wide again** — the queued goal
`class-scoped-types` named an uncreated rule by its `rule:` token inside a check's `argv`, which the
citation resolver reads like any other, so it named it by path instead. `session.py --wrap` works again;
the last sessions' by-hand commits were that red. Stage 1's floor is goal `m8-stdlib-depth`'s whole
list, carried and untouched.

## Next group

**Stage 4: the library — `Core\Compress`'s incremental surface** — one file set:
`crates/nvs-stdlib/src/compress.rs` and `tests/conformance/core/`.

- [ ] **`Core\Compress\Stream` inflates chunk by chunk under one bound** —
      `crates/nvs-stdlib/src/compress.rs:47`'s known gap 1: `deflate_init`, `deflate_add` and
      `inflate_init` map to a `Core\Compress\Stream` instance and none of them is written. Every backend
      beside it is driven whole on purpose (`crates/nvs-stdlib/src/compress.rs:471`), and the gap states
      the trap: a bound applied per call rather than per stream is not a bound
      (`rule:core-classes/decompression-bound`).
- [ ] **The conformance case the stage-4 check names** —
      `tests/conformance/core/compress-a-stream-inflates-chunk-by-chunk-under-one-bound.nvst`, which does
      not exist yet. The three cases beside it are all whole-buffer; the bound's own shape to copy is
      `tests/conformance/core/compress-refuses-a-decompression-past-its-bound.nvst:1`, and the new one
      asserts the bound across chunks rather than within one.

## Backlog

- The probe trace's cache half: a negative `PathEntry` per probed miss, and the trace's digest beside the
  content hash in the unit key — `crates/nvs-hir/src/requires.rs`'s known gaps own it,
  `rule:packaging/autoload-probes-fold-into-the-cache-key` specifies it, and `UnitKey::new` has two call
  sites (`crates/nvs-cli/src/script.rs:491`, `:546`).
- A discovery query's listed directories reach no revalidation set either — the same rule's second
  paragraph, untouched.
- A goal's check may not name a rule it has not created yet by its `rule:` token — the resolver reads
  every token under `docs/`. Nothing enforces that; `tools/rules.py`'s `--citations` is where it would go.
