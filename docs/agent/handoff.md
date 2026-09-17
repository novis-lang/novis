# Handoff

## State

**Goal `decided-closures`, stage 3 — the checker and the front end — is done**, and it landed as one
commit, `feat(hir): a class link under `Core` is checked against a roster…`. All three of that stage's
checks are green: `nvs-types`, `nvs-syntax`, and now `nvs-hir`, whose
`a_core_name_the_roster_lacks_is_refused_by_the_link_pass` was the stage's last red name.

A class link under `Core` no longer resolves by spelling. `nvs_hir::CoreRoster` is a two-case type —
`Names(&[&str])` or `Trusted` — held by `HierarchyResolver` from construction and asked in
`resolve_supertype`, so every link error still comes from that one pass. `Trusted` exists because a
bare empty slice would read as "the `Core` namespace declares nothing" and refuse every link in the
fixtures that resolve a file with no stdlib in hand: `resolve_file` is that door and its doc says so,
while `resolve_program`/`resolve_program_linted` carry the roster. The front ends pass
`nvs_stdlib::registry::link_targets` — `CLASSES` plus `DERIVE_INTERFACES` (`Core\Json\Codec`,
`Core\Db\Codec`, which `rule:core-classes/derive-attribute` says a class may write and no `CoreClass`
row holds) — and that function owns the bound on which names a roster holds.

**The tree does not build, and none of it is this goal's.** Another writer is mid-way through an
`nvs run --count` / `rule:testing/bench-counters` feature: `crates/nvs-runtime/src/ctx/{mod,trace,
wiring}.rs`, `crates/nvs-runtime/src/lib.rs`, `crates/nvs-cli/src/main.rs`, plus `tools/dossier.py`,
`docs/rules/testing/member-perf-ledger.md` and `benches/members/README.md`, all uncommitted. `nvs-cli`
fails to compile: `crates/nvs-cli/src/bundle.rs:119` calls `run_run` with 8 of its 9 arguments,
`crates/nvs-cli/src/main.rs:1139` has a pattern that does not mention `count`, and `main.rs:2463`
leaves `allocated_before` unused. **Do not finish or revert that work** — it is someone else's in
flight. This session therefore verified with `cargo test --workspace --exclude nvs-cli` (173 test
binaries, all ok), `cargo clippy --all-targets --workspace --exclude nvs-cli` (clean) and `cargo fmt
--all --check` (clean but for that writer's own files). The `.nvst` trees were not run: they need a
built `nvs.exe`.

`python tools/owners.py --closes decided-closures` names 33 gaps.

## Next group

**Stage 4: the library** — one file set: `crates/nvs-stdlib/src/zip.rs` and the `#[test]`s that pin it,
which `-p nvs-stdlib` runs. No rule owns `Core\Zip` (`brief.py --where zip` finds none), so each gap's
own `Decided:` sentence is the specification and `docs/agent/loop-goal.md` § *Stage 4* lists both.
Neither slice touches `nvs-cli`, so both are workable while the tree above is red.

- [ ] **`crates/nvs-stdlib/src/zip.rs:80` — read Zip64's extra fields and end-of-directory record.**
      Gap 1's `Decided:` sentence is the decision: an archive over 4 GiB or past 65535 entries writes
      `0xFFFFFFFF` in the fields this module reads and its real sizes in a zip64 extra field, and is
      refused as malformed today. The acceptance name is
      `a_zip64_archive_is_read_through_its_extra_fields_and_end_record`. Say what the second header
      format spends per archive in the module doc's *What this spends* paragraph
      (`rule:programs/memory-priority`), and delete the numbered item when it lands.
- [ ] **`crates/nvs-stdlib/src/zip.rs:88` — verify an entry's CRC by default and refuse a mismatch.**
      Gap 2's `Decided:` sentence, covering both `read` and `extract`; a stored entry is the case
      answered uncorrected today, since a corrupt deflate stream already fails to decode. The
      acceptance name is `an_extracted_entry_whose_crc_mismatches_is_refused`. The refusal's class
      comes from the module's existing `Fault::thrown_as` shape, and every reference card whose
      `errors` list changes is edited in the same slice (`docs/agent/conventions.md` § *A `Core`
      member*).

## Backlog

- `crates/nvs-stdlib/src/random.rs` gap 1 — `Core\Random\Seeded` registered per spec § 11, which the
  goal's § *Standing decisions* pre-authorizes against the sheet's recommendation.
- `crates/nvs-stdlib/src/uuid.rs:104` gap 1 — the bytes pair, a spec § 11 amendment.
- `crates/nvs-stdlib/src/path.rs` gap 1 — a UNC root as a third root shape.
- `crates/nvs-stdlib/src/mime.rs` gap 1 — a shared `Ebml` case.
- The prepared-pattern channel (`cldr.rs` gap 1 + `time.rs` gap 1) is the goal's one ADR slot; 0191 was
  the next free number at this commit — re-derive before claiming it.
- `docs/agent/loop-goal.md` § *Stage 4* is the full list of that stage's builds, strikes and deferrals.
