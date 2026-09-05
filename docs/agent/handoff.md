# Handoff

## State

**Goal 21, stage 3's first half has landed whole**, and with it the thing that mattered more: the
acceptance run had been producing **no measurement at all**. `examples/stream.nvs` (stage 5) and
`examples/octets.nvs` (stage 10) were absent from the `files` list `tools/loop.py`'s `begin` walks
before it builds anything, so every check behind them was unreached — the playbook's own
*A missing entry in `loop-goal.toml`'s `files`* bullet, hit again. Both now exist as stand-ins that
compile, run and print what is true, so only their own `exact` checks fail and every other stage is
measured. Their source is not frozen; each names in a comment the program that replaces it.

**ADR 0067 § 3's `db.open` wildcard now has a reader on both sides.** `host_granted`
(`crates/nvs-config/src/capability.rs:307`) is the single home and takes the `Cap`, because
`Cap::takes_host_wildcard` decides whether an entry is a pattern at all and the compiler's asker and
the runtime's must not answer that differently either (ADR 0057 § 4). A pattern-looking entry under
`db.open` is *only* read as a pattern and never falls back to an exact match, so `*` and `*.` grant
nothing at all. The rule and its three refusals are that module's own doc, § *A `db.open` entry may
be a `*.` wildcard*; ADR 0067 § 3 states it too, folded in under the goal's standing decision.
Stage 3's three items collapsed into one slice — they are one predicate and one test file.

Nothing is blocked. The goal's item 5 gives a false *reason* for `net.connect` taking no wildcard;
the playbook's new bullet has it, and the ruling itself is untouched.

## Next group

**Stage 3's second half: `nvs check` reads the configuration, so ADR 0067 § 10's host diagnostic
fires for somebody.** The decision is pre-authorized in the goal's § *Standing decisions* (item 6).
One file set: `crates/nvs-cli/src/main.rs`, `crates/nvs-types/src/lib.rs`, and a new
`crates/nvs-cli/tests/check.rs` (the crate keeps no in-file test modules).

- [ ] **`front_end` builds an `Env` with grants in it** — `crates/nvs-cli/src/main.rs:717` is
      `front_end` and builds none, `crates/nvs-cli/src/main.rs:552` is where `Command::Check`
      dispatches to `run_check`, `crates/nvs-types/src/lib.rs:404` is the `Env::grants` field and
      `crates/nvs-types/src/check.rs:108` is `check_program_granted`, the entry a caller that has
      one uses. Resolve `nvs.toml` exactly as `nvs run` does. Test:
      `nvs_check_reports_an_open_host_no_grant_covers`.
- [ ] **No configuration found means no grants, silently** — today's behaviour, kept: a tree with no
      `nvs.toml` reports nothing about capabilities. Same anchors,
      `crates/nvs-cli/src/main.rs:717`. Test: `nvs_check_with_no_config_reports_no_grant_diagnostic`.
- [ ] **A malformed `nvs.toml` fails `nvs check` with that config error**, not with a program
      diagnostic — a command that reads configuration is a command a broken configuration can fail.
      `crates/nvs-cli/src/main.rs:552`. Test: `nvs_check_reports_a_broken_nvs_toml_as_a_config_error`.

Strike `carried-gaps.md`'s `nvs check` row and `crates/nvs-types/src/intrinsics.rs` gap 6 when it
lands.

## Backlog

- Stage 5 — `Core\Db::stream`/`streamAs`/`close`, `crates/nvs-stdlib/src/db/mod.rs` gap 5. Its
  fixture is a stand-in naming the three frozen lines it owes.
- Stage 10 — `Core\Uri`'s two decoders answer `bytes`, `crates/nvs-stdlib/src/uri.rs`. Same.
- `tools/chain.py --check` does not read `carried-gaps.md`; stage 2's second check reads as though
  it does. Either the tool gains the walk or the check's name is narrowed.
- `orient.py` warned that `[context] modules`' pattern `crates/nvs-stdlib/tests/spec_registry_coverage.rs`
  matched no module: the map is built from `src/` module docs, so a `tests/` path can never match.
- Goal 27 is the same column one level down — ~110 module-doc `# Known gaps` items with no owner
  ([carried-gaps.md](carried-gaps.md), last *Owned* rows).
