# Handoff

## State

**Goal `config-is-written` — stage 1 is green, so with stage 2 already green the goal's own red
stages are 3 and 4.** `crates/nvs-cli/src/cache.rs` now carries the artifact cache's pair:
`the_configured_cache_directory_is_read_from_the_key_the_tree_documents` parses `[opcache]
file_cache_dir` through `nvs_config::file::parse` and asserts `from_config`'s cache is rooted
exactly there, and `a_cache_directory_written_in_the_retired_spelling_is_reported_not_ignored`
holds `[cache] dir` to `E0601` naming both the key and the block it sits in.

**Stage 1 needed no new behaviour.** `opcache.file_cache_dir` was already the only spelling
(`docs/decisions/0175.md` § 2, § 4); what was missing was the two cases, and three comments in
`cache.rs` that still called the cache root `[cache] dir` after that record retired the key.

**Stage 3 is the goal's remaining implementation** — a project command that resolves no tree
writes the shipped default file and then resolves it. There is no `nvs init` yet:
`crates/nvs-cli/src/main.rs:911`'s arm set has no `Command::Init`.

## Next group

**Stage 3: a project command with no tree writes one — one file set:**
`crates/nvs-cli/src/config.rs` and `crates/nvs-cli/src/main.rs`, with
`crates/nvs-config/src/lib.rs:112`'s `default_file()` read only.

- [ ] **The write itself, gated on which command is running.**
      `crates/nvs-cli/src/config.rs:166` is the single call that owns all three steps of
      `rule:config/the-root-is-config-else-nvs-toml-else-the-shipped-defaults`, so reaching step 3
      is what the write hangs off; `crates/nvs-cli/src/config.rs:150`'s `boot_origins` does not
      know which subcommand is running, so the gate is a parameter the arms at
      `crates/nvs-cli/src/main.rs:911` fill with the five the goal's § *Standing decisions* tables
      (`run`, `serve`, `test`, `build`, `check`). Check first whether
      `nvs_config::resolve::Roots` already reports which step won — if it does not, that is the
      one addition `nvs-config` owes this slice.
- [ ] **`a_run_in_a_directory_with_no_config_writes_one_and_then_resolves_it` and
      `an_existing_nvs_toml_is_never_touched`.** Both drive the built binary in a scratch
      directory, which is `crates/nvs-cli/tests/meta.rs`'s shape — not the `#[cfg(test)]` unit
      shape `crates/nvs-cli/src/cache.rs:1422` uses, since nothing here is `pub(crate)`.
- [ ] **`the_written_file_is_the_shipped_template_byte_for_byte`.** What lands on disk is
      `crates/nvs-config/src/lib.rs:112`'s `default_file()` verbatim, with no rendering step
      between the `include_str!` and the write.

## Backlog

- `nvs init` needs a new `Command::Init` — stage 3's third check, `crates/nvs-cli/src/main.rs:911`.
- Stage 3's four refusals, including the environment opt-out — `docs/agent/loop-goal.md` stage 3.
- Stage 3's info record for a write that cannot happen — goal § *Standing decisions*, the silent-
  failure entry.
- Stage 4: the rule fragment and the record for the write — `docs/rules/config/`, `docs/decisions/`.
