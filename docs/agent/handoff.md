# Handoff

## State

**Goal `config-is-written` — stage 2 is green, and stage 1's `nvs-config` half is now green too.**
`crates/nvs-config/tests/directives.rs` carries the roster's two Rust assertions:
`every_unread_key_names_what_is_missing_and_who_owns_it` holds each `[unread:]` trailer to a
non-empty *why* and an owner that can be opened — a `rule:` id or a four-digit record number — and
`no_key_with_a_reader_still_claims_to_be_unread` holds every key the default file marks
`NOT IMPLEMENTED` to having no reader that spells it under `crates/*/src` or `benches/*/src`.

**The reader half is the narrower of the two claims its name allows, and its doc comment says so.**
A reader is counted three ways (`tools/directives.py`'s module doc) and only the dotted key as a
string literal is a spelling a scan can answer exactly: the field *name* is not the key, because
`[metrics] listen` and `[debug] mode` share their names with `capabilities.net.listen`,
`server.listen` and `[app] mode`. The census over all three spellings stays the tool's, which
`verify.py` runs — and the half asserted in Rust is the one a field search cannot see.

**The dotted key comes off the default file's own block headers**, so neither case walks the field
graph: `settings_with_prose()` now returns the header with each setting, and the pair of cases that
was already there holds that marked set equal to the tree's trailers.

**Stage 1's remaining check is `nvs-cli`'s, and it is a test over landed behaviour rather than new
code.** `opcache.file_cache_dir` is already the only spelling of the artifact cache directory
(`crates/nvs-config/src/tree.rs:966`, `docs/decisions/0175.md`), `from_config` already reads it, and
`[cache]` has no `dir` field at all — so what is missing is the pair of cases that pins both.

## Next group

**Stage 1: the artifact cache's one spelling, in `nvs-cli` — one file set:**
`crates/nvs-cli/src/cache.rs`, with `crates/nvs-config/src/tree.rs` read only.

- [ ] **`the_configured_cache_directory_is_read_from_the_key_the_tree_documents`.**
      `crates/nvs-cli/src/cache.rs:1331`'s `from_config` builds the cache at the path
      `opcache.file_cache_dir` names and `crates/nvs-cli/src/cache.rs:989`'s `dir()` hands it back,
      so the case sets that key on a `Config` and asserts the path it gets. **`from_config` is
      `pub(crate)`**, so this cannot be an integration test under `crates/nvs-cli/tests/` — it is a
      `#[cfg(test)]` unit test in `cache.rs`, which the check's `cargo test -p nvs-cli` runs either
      way. Specified by `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` and the
      goal's stage 1.
- [ ] **`a_cache_directory_written_in_the_retired_spelling_is_reported_not_ignored`.** `dir` is not
      a field of `crates/nvs-config/src/tree.rs:971`'s `Cache`, so `deny_unknown_fields` refuses
      `[cache] dir` already; the case parses that file text through `nvs_config::file::parse` and
      asserts the refusal *names* the key, rather than a boot taking it silently. `nvs-cli` depends
      on `nvs-config`, so it can host this beside the case above.

## Backlog

- Stage 3, the write: six tests over `nvs-cli`, four of them refusals — `docs/agent/loop-goal.toml`
  stage `3 the write`.
- `crates/nvs-config/src/default.toml` carries no `[db]`/`[mail]`/`[storage]` block name but one
  example each (`main`, `default`, `local`); if a doc wants the canonical example names, that is
  `docs/reference/tools/20-config.md`'s to state.
- The field-access and bare-name reader spellings have one home, `tools/directives.py`; a Rust case
  that re-derives either answers about whichever same-named field it found.
