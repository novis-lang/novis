# Handoff

## State

**Goal `config-is-written` — stage 2 is green.** `crates/nvs-config/src/default.toml` is on disk with
every one of the tree's 212 leaf keys spelled once and commented out, `nvs_config::default_file()`
hands it to a caller, and the three tests stage 2 names pass. `python tools/directives.py
--check-template` is green and is now step 4 of `verify.py`, so the file and the tree cannot drift
outside the loop either.

**Stage 1 is what is still open, in two checks, and neither is a regression** — both name tests no
crate has written yet. `nvs-cli` owes the artifact cache's one-spelling pair, and `nvs-config` owes
the roster's two Rust assertions; the ledger line the driver repeats is the first of those.

**The file states a default only where the tree states one.** A value beside a key is that key's
*spelling*; a claim about the shipped default appears only in a sentence opening `Unset,`, and the
preamble says so. The three places one was derivable: `crates/nvs-config/src/mode.rs:69`'s `DERIVED`
table (`[log] format` is `json`, `[log] level` is `Info`, `[http.errors] detail` is `generic`, each
the production value a reader must assume because the table is not applied at boot),
`crates/nvs-runtime/src/ctx/limits.rs:376` (an unset `[limits]` key is **no cap at all**, which is
why that block claims none), and `DEFAULT_MAX_SCRIPT_DEPTH = 64`.

**`[db.pool]`'s four bounds are the one place the file names keys an operator should not write.** The
tree parses them, so `--check-template` requires them spelled; `nvs_config::db::validate` refuses a
table of bounds written unscoped. The prose above them says both things and points at
`[db.<name>.pool]`.

## Next group

**Stage 1: the roster's two Rust assertions, which `directives.py` already makes and no `cargo test`
does — one file set:** `crates/nvs-config/tests/directives.rs`, `crates/nvs-config/src/tree.rs`.

- [ ] **`every_unread_key_names_what_is_missing_and_who_owns_it`.** `declared_unread()` at
      `crates/nvs-config/tests/directives.rs:401` already parses every `[unread:]` trailer out of
      `tree.rs`; what this adds is the assertion that each one names a non-empty *why* and an owner
      that is a `rule:` id or a record number, the shape `tools/directives.py:515` refuses at.
      Specified by `rule:config/three-changeability-classes`' roster half and the goal's stage 1.
- [ ] **`no_key_with_a_reader_still_claims_to_be_unread`.** The half that needs a reader census, which
      is a grep over the workspace (`tools/directives.py:515` is the same assertion in Python).
      **Decide first** whether the Rust case walks `crates/**/*.rs` itself or asserts the narrower
      thing it can see — the trailer set at `crates/nvs-config/src/tree.rs:379` against the keys the
      crate's own modules read — and say which in the case's doc comment, because the narrower one is
      a different claim from the check's name.

## Backlog

- Stage 1's other half: the artifact cache's one spelling, `crates/nvs-cli/src/cache.rs` and a
  `[unread:]`-free reader for `opcache.file_cache_dir` — ADR 0175 is the decision, the goal's stage 1
  is the list.
- Stage 3, the write: six tests over `nvs-cli`, four of them refusals — `docs/agent/loop-goal.toml`
  stage `3 the write`.
- `crates/nvs-config/src/default.toml` carries no `[db]`/`[mail]`/`[storage]` block name but one
  example each (`main`, `default`, `local`); if a doc wants the canonical example names, that is
  `docs/reference/tools/20-config.md`'s to state.
