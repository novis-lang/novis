# Handoff

## State

**Spec § 1 is four rows shorter.** `Core\Str::graphemes`, `codePoints`, `fromCodePoint` and
`fromCodePoints` are registered, implemented and pinned by two conformance cases. `graphemes` and
`codePoints` are `granularity::Unit::{Grapheme, CodePoint}.pieces()` written out as members, so the
class's own unit and the scalar-value unit are each reachable on purpose rather than by accident;
`fromCodePoints` is `codePoints`'s inverse and the pair round-trips. The refusal both `from*` members
share — above U+10FFFF, and the surrogate range — is `str.rs`'s `scalar_value`, which throws rather
than substituting, per ADR 0009 § 1.

**The loop's gate is the ratchet in `crates/mwl-stdlib/tests/spec-members-outstanding.txt`**, now at
**40 keys** for §§ 1-12; when it is empty, Part I is registered whole. That test's own module doc says
why it is a ratchet rather than a red assertion, and what its parser deliberately does not reach.
Conformance is **389** of 600; differential is 86 of 150 and has not moved.

**`examples/collect.mwl`'s frontier is unchanged** — `Core\Uri::parseQuery` at `collect.mwl:36`, then
`Core\Csv::parse`, `Core\Validate::isEmail`, `Core\Out::capture`.

**§ 1 now owes five rows**: `compare`, `replaceAll`, `replaceRange`, and the `fold`/`normalize` pair,
which needs a Unicode-normalization dependency picked under ADR 0051 § 4 and is therefore its own
group.

## Next group — the rest of § 1 that needs no new dependency

Three slices, all in one file, each striking its own line from the outstanding list. `fold` and
`normalize` are deliberately not here: pick their dependency in a group of its own.

**Shared file set:** `crates/mwl-stdlib/src/str.rs` (`:49` `CLASS`'s rows — `:183` `replace` is the
neighbour all three belong beside; `:416` `address()`; `:1685` the test module),
`crates/mwl-stdlib/tests/spec-members-outstanding.txt`, and `tests/conformance/core/`.

- [ ] **`Core\Str::replaceAll`** (`01-core-library.md` § 1 *Transformation*, `:151`).
      `replaceAll(string $s, array<string> $pairs, {caseInsensitive?: bool}): string` replaces
      `str_replace`-with-arrays and `strtr` at once. The `$pairs` array is keyed *needle → replacement*,
      so the helper iterates keys as well as values — `mwl_array_next_slot` gives the slot and
      `mwl_runtime`'s key read gives the needle; `mwl_core_str_join` at `:634` is the value-only
      shape to extend. Decide and state in the doc comment whether a replacement can itself be
      re-matched (`strtr` says no, longest-needle-first; `str_replace` says yes, in order) — `strtr`'s
      reading is the one that does not depend on argument order. Strike `§1 replaceAll`.
- [ ] **`Core\Str::replaceRange`** (§ 1 *Transformation*, `:152`). `replaceRange(string $s, int
      $offset, ?int $length, string $replacement): string` — `substr_replace`. `CoreTy::Nullable`
      exists (`registry.rs:202`) and this is its first use in `Core\Str`, so check what a written
      `null` argument arrives tagged as before writing the case. The offset is ADR 0063 R8's
      negative-from-the-end rule, and the unit is `granularity::DEFAULT` like every other range here;
      `mwl_core_str_slice` already resolves an R8 range and is the body to reuse. Strike
      `§1 replaceRange`.
- [ ] **`Core\Str::compare`** (§ 1 *Comparison*, `:110`). `compare(string $a, string $b,
      {caseInsensitive?: bool, natural?: bool}): int` replaces `strcmp`, `strcasecmp`, `strnatcmp`,
      `strnatcasecmp` and the comparator behind `natsort`. Answer `-1`/`0`/`1` rather than a byte
      difference, and say so in the doc comment: PHP 8 already normalized this and a program that
      subtracted the old magnitude was always wrong. `natural` is a digit-run comparison written here,
      not a dependency. Strike `§1 compare`.

## Backlog

- `fold`/`normalize` need a Unicode-normalization crate picked under ADR 0051 § 4 — `docs/adr/0051`.
- `Core\Uri::parseQuery` is `collect.mwl`'s first stop; its return type is settled in
  `docs/agent/loop-goal.md` § *Standing decisions*.
- § 2 owes `Arr::diff`/`intersect` and ADR 0069's combination members — `docs/adr/0069`.
- § 9 owes `Core\Heap` and the `Iterable` its rows declare — `docs/spec/01-core-library.md` § 9.
- § 11 owes `Random::bytes` and `Hash::stream`; the runtime `bytes` tag they needed exists now.
- ADR 0088's qualifier classification is registry-wide and unstarted — `docs/implementation-plan.md`.
