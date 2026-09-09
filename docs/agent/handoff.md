# Handoff

## State

**Goal 25's Stage 4 has landed its reader half: `Core\Zip` is registered, and the three refusals
happen when the archive is read.** `entries(bytes $archive): array<tainted string>` and
`read(bytes $archive, string $name, uint $maxBytes = 67108864, uint $maxRatio = 1000): tainted
bytes` are the surface on disk; `crates/nvs-stdlib/src/zip.rs:284`'s `directory` is the one route to
an entry, so `entries` and `read` share every refusal and no member hands a program an entry it
would have had to check itself. Both answers are unconditionally `tainted` and are now rows of
`nvs_types::core_lib`'s closed roster — `crates/nvs-types/src/core_lib.rs:1271` is the gate a new
tainted-answering member has to pass, and it fails the build rather than the review.

**Four refusals, not three.** The goal names traversal, absolute path and symlink; an archive naming
one entry twice is refused beside them, because which of the two a reader answers is the whole of
that attack and the priority ordering does not trade it. The absolute and `..` tests are
`crate::path::parse`'s — widened to `pub(crate)` in this slice — so the grammar is `Core\Path`'s one
grammar on every platform rather than a second copy, per
`rule:security/path-scope-canonicalise-then-prefix`'s "there is one implementation of this rule".

**The bomb is `compress::Bound` and no new arithmetic**: `output_ceiling` is now `pub(crate)` and
`crates/nvs-stdlib/src/zip.rs:413`'s `contents` compares against it while the output grows. Only the
read loop and the messages are local, so a refusal says `Core\Zip` and names the entry.

**What is not written is extraction**, which is where the bound is charged *across* the archive and
where a destination is checked after resolution — `crates/nvs-stdlib/src/zip.rs`'s known gap 1, and
the next group below. Two acceptance tests in `docs/agent/loop-goal.toml`'s `4 zip` check stay red
until it lands: `a_decompression_bomb_is_refused_by_stage_twos_bound_per_entry_and_across_the_archive`
and `extraction_cannot_escape_its_destination_when_a_symlink_appears_during_it`. No ADR was opened,
per the goal's standing decisions; § 17's `Core\Zip` row now carries the surface and names `extract`
as designed.

## Next group

**Stage 4: `Core\Zip::extract`, where the destination is checked after resolution** — one file set:
`crates/nvs-stdlib/src/zip.rs`, `crates/nvs-stdlib/src/registry.rs`,
`tests/conformance/core/zip-*.nvst`. `crates/nvs-stdlib/src/storage.rs:105` is the freshest shape for
a member that goes through a capability door, and every door it needs already exists —
`nvs_runtime::capability::create_dir` at `crates/nvs-runtime/src/capability.rs:472`, `write` at
`crates/nvs-runtime/src/capability.rs:391`, `canonicalize` at
`crates/nvs-runtime/src/capability.rs:705`.

- [ ] **The bomb across the archive as well as per entry**, so an archive whose entries are each
      within the bound and whose total is not is refused — `rule:core-classes/decompression-bound`,
      which states both halves as one rule. The accumulator belongs where the per-entry ceiling is
      already computed, `crates/nvs-stdlib/src/zip.rs:413`, by handing each entry a `Bound` whose
      `bytes` is what the archive has left. Test:
      `a_decompression_bomb_is_refused_by_stage_twos_bound_per_entry_and_across_the_archive`.
- [ ] **`extract(bytes $archive, string $destination, ...): uint`, checked after path resolution**
      rather than before, so a symlink that appears during the extraction cannot win the race —
      `rule:security/path-scope-canonicalise-then-prefix`, whose "one implementation" sentence means
      the resolution is `nvs_runtime::capability::canonicalize`'s and only the containment
      comparison is local. Row and card beside `read` at `crates/nvs-stdlib/src/zip.rs:90`; the
      `fs.write` row goes in `crates/nvs-stdlib/src/registry.rs:1922`'s `CAPABILITIES`. Test:
      `extraction_cannot_escape_its_destination_when_a_symlink_appears_during_it`.
- [ ] **A fourth `.nvst` case for `extract`, and the fixture grows the extraction half.** A case
      that extracts needs a directory, so it is a multi-file case
      (`docs/agent/playbook.md` § *Writing a test case*). The archives to reuse are already base64
      in `tests/conformance/core/zip-refuses-a-hostile-entry-when-the-archive-is-read.nvst:9`, and
      `examples/zip-refuses.nvs:74` is where the fixture ends at the bound today and the
      destination half belongs.

## Backlog

- Stage 5: `docs/spec/02-php-migration.md` has no `fileinfo` and no `zip` rows at all, so both are a
  write rather than an edit — `docs/agent/loop-goal.md` § *Stage 5*.
- `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` is down to four keys; `§17 Core\Xml`
  is goal 26's and `§16 Core\Metrics` is unowned (`docs/agent/carried-gaps.md` § *Unowned*).
- Zip64 and the CRC check are `crates/nvs-stdlib/src/zip.rs`'s known gaps 2 and 3; neither is on this
  goal.
