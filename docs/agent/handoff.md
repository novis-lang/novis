# Handoff

## State

**Goal 25's Stage 2 has landed whole: `Core\Compress` is registered, implemented, bounded and
pinned.** `compress` and `decompress` are the whole-buffer pair, `Core\Codec` is **five** cases and
not four — the migration table's `gzencode`/`gzcompress`/`gzdeflate` rows name three distinct byte
formats over one deflate stream — and the incremental half (`deflate_init`, `deflate_add`,
`inflate_init`) is `crates/nvs-stdlib/src/compress.rs`'s known gap 1 rather than an open question.

**The bound is `rule:core-classes/decompression-bound`, opened as ADR 0166** — the one ADR number
this goal authorised. Two numbers, `[limits] max_decompressed` (64 MiB) and
`[limits] max_decompression_ratio` (1000:1), both `System`-class, both clamping a call's own
`$maxBytes`/`$maxRatio` downward only, and neither with a spelling for "off": `false` reads as the
shipped default. A breach is a `ParseError` naming the rule, never an `IOError`.

**Stage 0's § 17 rows are done for two of the three classes** — `Core\Compress` and `Core\Zip` now
carry the surface and the ADR; `Core\Mime`'s row is still one line with no ADR. **Stage 5's first
strike is already spent**: `every_part_two_spec_class_is_registered` refuses a line naming a
registered class, so `§17 Core\Compress` left
`crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt` with the class rather than at the
end of the goal — and Stage 3 and Stage 4 each owe theirs the same way. The migration table's
`zlib` rows are still Stage 5's. Dependencies are `flate2`, `brotli` and `ruzstd`, all pure Rust,
notice regenerated.

## Next group

**Stage 3: `Core\Mime`, magic bytes and no rule interpreter** — one file set: the new
`crates/nvs-stdlib/src/mime.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-stdlib/src/lib.rs`. `crates/nvs-stdlib/src/compress.rs` is the shape to copy for all
three: a class const, an enum const, a `MethodDoc` per row, an `address` arm, and a `#[cfg(test)]`
module holding the acceptance names.

- [ ] **Detection is from magic bytes and never from a file extension**
      (`rule:core-api/tier-roster`'s § 17 row; a type read off an extension is a type an attacker
      chose). A fixed table this crate carries, not libmagic's rule language. Register the class at
      `crates/nvs-stdlib/src/registry.rs:1695`, beside `crate::compress::CLASS`, and the module at
      `crates/nvs-stdlib/src/lib.rs:203` and `crates/nvs-stdlib/src/lib.rs:370`. Test:
      `a_type_is_detected_from_magic_bytes_and_never_from_a_file_extension`.
- [ ] **The answer is a closed enum plus an `Unknown`, never a free string**, so a caller cannot
      compare against a spelling that never occurs — the same decision `Core\Codec` makes at
      `crates/nvs-stdlib/src/compress.rs:83`, whose ordinals-are-ABI note applies here too. Register
      it beside `crate::compress::CODEC` at `crates/nvs-stdlib/src/registry.rs:2389`. Test:
      `the_answer_is_a_closed_enum_plus_unknown_and_never_a_free_string`.
- [ ] **Detection launders nothing** (`rule:security/launderers-are-sink-named`): bytes that detect
      as `image/png` are still `tainted`, so the subject parameter is
      `CoreTy::Blob(Qual::Contagious)` — `crates/nvs-stdlib/src/compress.rs:138` is the worked
      example — and the sentence goes in the member's own reference card, which is where a caller
      will read it. Test: `detected_bytes_are_still_tainted_because_detection_is_not_a_launderer`.
- [ ] **Three `.nvst` cases, and the strike in the same slice.**
      `conformance_coverage.rs`'s floor is three cases per member, and its error-path gate wants
      every `Fault::` site either reached by a case or declared "unreachable from source" within
      eight lines above it. `§17 Core\Mime` leaves
      `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:18` in the slice that
      registers the class, not at Stage 5: `every_part_two_spec_class_is_registered` fails on a
      line naming a class that is registered now.

## Backlog

- Stage 4, `Core\Zip`: four hostile archives, four refusals, and the bound above applied per entry
  *and* across the archive — ADR 0166 § 6 says it gets no second rule (`docs/agent/loop-goal.md`).
- Stage 5: strike `§17 Core\Compress`, `§17 Core\Mime` and `§17 Core\Zip` from
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt`, and answer the migration table's
  `zlib`, `fileinfo` and `zip` rows (`docs/spec/02-php-migration.md`).
- `Core\Compress`'s incremental half — `Core\Compress\Stream`, shaped like `Core\Hash\Stream` —
  is known gap 1 in `crates/nvs-stdlib/src/compress.rs`, and no goal carries it yet.
- `python tools/records.py --check` reports four records missing `modifies: []`; 0163, 0164 and 0165
  are another goal's and were left alone.
