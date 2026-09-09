# Handoff

## State

**Goal 25's Stage 3 has landed whole: `Core\Mime` is registered, its table is compiled in, and its
three acceptance tests are green.** `detect(bytes $data): Mime\Type` and
`mediaType(Mime\Type $type): string` are the whole surface. `Core\Mime\Type` is **17** cases whose
zero is `Unknown`, and `crates/nvs-stdlib/src/mime.rs`'s `SIGNATURES` is 22 entries of literal octet
runs at fixed offsets, matched by comparison — no search, no offset read out of the input, no
arithmetic, which is the half of libmagic the module doc says this class declines.

**The subject parameter is `Qual::Neutral`, not the `Contagious` the previous group's item
proposed.** `detect` answers an enum case and a case carries no octet of its subject, so nothing
qualified can reach the answer; `Neutral` is the mark that accepts a tainted argument without
laundering it. What survives is the *argument's* mark, and no passing program can show that — the
pin is a refusal, `tests/conformance/core/mime-detection-launders-nothing-it-detected.nvst`.

**No ADR was opened**, per the goal's standing decisions: the surface is a `rule:core-api/tier-roster`
row, and § 17's `Core\Mime` row now carries it. That rule's "designed, not shipped" list named five
classes that are registered — the networking, OS, signal and compression ones — and now names only
the archive class, XML, big integers, signature and metrics.

**Stage 5's second strike is spent**: `§17 Core\Mime` left
`crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` in the registering slice, because
`every_part_two_spec_class_is_registered` refuses a line naming a registered class. `§17 Core\Zip`
is Stage 4's. `docs/spec/02-php-migration.md` has **no `fileinfo` rows at all** — no
`mime_content_type`, no `finfo_*`, no `exif_imagetype` — so Stage 5's "the `fileinfo` rows answer a
`Core` spelling" is a write rather than an edit.

## Next group

**Stage 4: `Core\Zip`, where the policy lives** — one file set: the new
`crates/nvs-stdlib/src/zip.rs`, `crates/nvs-stdlib/src/registry.rs`, `crates/nvs-stdlib/src/lib.rs`,
`Cargo.toml`, `examples/zip-refuses.nvs`. `crates/nvs-stdlib/src/mime.rs` is the freshest shape to
copy, and the bomb half is **reused and never restated**: `compress::Bound` is
`crates/nvs-stdlib/src/compress.rs:295`, its ceiling `crates/nvs-stdlib/src/compress.rs:314` and its
one-way clamp `crates/nvs-stdlib/src/compress.rs:328`.

- [ ] **Three refusals, at read time, before any byte reaches a path**: a traversing entry, an
      absolute-path entry, a symlink entry (`rule:security/a-path-is-not-a-url` for what a path is;
      the goal's Stage 4 owns the three). Refused by the reader so a program cannot opt out by
      walking entries itself. Register the class at `crates/nvs-stdlib/src/registry.rs:1701`, beside
      `crate::mime::CLASS`, and the module at `crates/nvs-stdlib/src/lib.rs:234` and
      `crates/nvs-stdlib/src/lib.rs:393`. Tests:
      `a_traversing_entry_is_refused_at_read_time_by_the_diagnostic_that_names_the_rule`,
      `an_absolute_path_entry_is_refused_the_same_way`, `a_symlink_entry_is_refused_the_same_way`.
- [ ] **The bomb is Stage 2's bound applied per entry *and* across the archive**, not a second rule
      — `rule:core-classes/decompression-bound`, whose `Bound` is at
      `crates/nvs-stdlib/src/compress.rs:295`. A refusal is a `ParseError` naming the rule, never an
      `IOError`. Tests:
      `a_decompression_bomb_is_refused_by_stage_twos_bound_per_entry_and_across_the_archive`,
      `a_refusal_is_a_diagnostic_and_never_a_failed_io_error`.
- [ ] **Extraction takes a destination and never escapes it, checked after path resolution** so a
      symlink appearing mid-extraction cannot win the race. `crates/nvs-stdlib/src/path.rs` and
      `crates/nvs-stdlib/src/storage.rs` are the manifest's named modules for the containment and
      the grant; the class is registered at `crates/nvs-stdlib/src/registry.rs:1701`. Test:
      `extraction_cannot_escape_its_destination_when_a_symlink_appears_during_it`.
- [ ] **The fixture, three `.nvst` cases, and the strike in the same slice.**
      `examples/zip-refuses.nvs` is the acceptance check that is currently red — the goal's `files`
      list at `docs/agent/loop-goal.toml:10` — and its expected output is frozen in the `[[check]]`
      that runs it. `§17 Core\Zip` leaves
      `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:17` here, not at Stage 5, for
      the reason `Core\Mime`'s line did.

## Backlog

- Stage 5: answer `docs/spec/02-php-migration.md`'s `zlib`, `fileinfo` and `zip` rows — the
  `fileinfo` ones do not exist yet and have to be written.
- Stage 5: `the_server_still_sets_no_content_encoding_of_its_own` in `-p nvs-server`, which nothing
  has written (`docs/agent/loop-goal.toml`, stage `5 registered`).
- `Core\Compress`'s incremental half — `Core\Compress\Stream` — is known gap 1 in
  `crates/nvs-stdlib/src/compress.rs`, and no goal carries it.
- `Core\Mime`'s table grows by appending: EBML, BMP and every text format are deliberately absent,
  and `crates/nvs-stdlib/src/mime.rs`'s known gaps say why.
- `python tools/records.py --check` reports four records missing `modifies: []`; 0163, 0164 and 0165
  are another goal's and were left alone.
