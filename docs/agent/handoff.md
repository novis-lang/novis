# Handoff

## State

**ADR 0042 is wired at every producer of a unit, and measured.** `cache::unit_for`
(`crates/nvs-cli/src/cache.rs:1193`) is the whole decision and all four call sites make it:
`run_run`, `runner::compile` for `nvs test`, and `script::Compiler::compile`, which is the one a
`spawn script` isolate and `serve`'s per-core compiler both resolve through. Each resolves its
configuration snapshot *above* the compile, because both halves of the key are configuration — that
ordering is the wiring, and `cache::from_config` is the only place it is read.

**Stage 4's margin is on disk.** `a_warm_start_is_faster_than_a_cold_one_by_the_margin_this_test_names`
(`crates/nvs-cli/src/cache.rs:2413`) lowers a 160-function program once, above both arms, and
measures the fastest of five: 96ms cold against 10ms warm in a debug build here, asserted at 4x. The
front end is inside neither arm and the cold arm carries § 4's publish, both of which its own doc
says.

**One user-visible change:** `nvs test <program>` now refuses a tree that does not resolve, exactly
as `nvs run` does. Nothing is blocked.

## Next group

**What the wiring is not yet asserted to do.** One file set — `crates/nvs-cli/src/runner.rs`,
`crates/nvs-cli/src/script.rs`, `crates/nvs-cli/src/cache.rs`.

- [ ] **One program, two subcommands, one artifact**: a test that a `nvs run` and a `nvs test` of the
      same program address the same key, which is the claim `runner::compile`'s comment now makes
      (`crates/nvs-cli/src/runner.rs:559`). Both lower through `ENTRY_SCRIPT_LABEL` over the same
      file list, so the assertion is `cache::program_digest` equality plus a `Provenance::Loaded` out
      of the runner's compile over a cache the run populated.
- [ ] **A `spawn script` isolate warms across processes**: two `Compiler`s over one path sharing a
      cache directory, the second reading the artifact the first published
      (`crates/nvs-cli/src/script.rs:400`). The in-memory `units` map hides this inside one process,
      so the second `Compiler` is the whole point; `compiles` still counts a warm hit, because § 2's
      front end really did run, and what the test needs is a `Provenance` the resolver does not yet
      hand back.
- [ ] **Narrow or drop `#![allow(dead_code)]`** now that every site is wired
      (`crates/nvs-cli/src/cache.rs:170`): what is left over is the vocabulary only this module's
      tests reach, and the comment above the attribute says so rather than listing it.

## Backlog

- ADR 0042's "§ *Verification*" is the trailing block of `## Revisiting`
  (`docs/adr/0042-on-disk-artifact-cache-format.md:356`), not a heading — § 2 and this goal both cite
  it as a section and a session will go looking for one.
- `env_hash` spells "the compiler build" as the package version; `cache::default_dir` compensates and
  a configured `opcache.file_cache_dir` does not (`crates/nvs-cli/src/cache.rs`'s module doc).
- `nvs test`'s new refusal of an unresolvable tree is asserted nowhere (ADR 0078 § 1).
