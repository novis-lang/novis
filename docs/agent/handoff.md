# Handoff

## State

**The loop's own gate exists.** `crates/mwl-stdlib/tests/spec_registry_coverage.rs` parses the
`| Member | Signature | … |` rows out of `docs/spec/01-core-library.md` §§ 1-12 and checks each
against `registry::CLASSES`. It is a **ratchet**, not a permanently red assertion: the members still
owed live in `crates/mwl-stdlib/tests/spec-members-outstanding.txt` and the test fails both on an
unregistered member the file does not list *and* on a line whose member is registered now. That
file's **44 keys are the machine-readable work list for §§ 1-12**; when it is empty, Part I is
registered whole. Why a ratchet rather than a red test — `verify.py` stops at the first failing step,
so a red `cargo test` would cost every later session its clippy and fmt signal — is in that test's own
module doc, along with what the parser deliberately does not reach (`Core\Bytes`'s prose list, § 9's
`| Type | Members |` table, and members a Notes cell mentions in passing).

**It measures registration, not depth.** `Core\Json::decodeAs` has a row, so the gate is satisfied by
it while `json` gap 2 is still open. Behaviour is `conformance_coverage.rs`'s half of the pair.

**`Core\Str::chunk` and `Core\Str::lines` landed**, with `line_pieces` (`str.rs:782`) owning the
three-terminator rule and the no-trailing-empty-line decision. Conformance is **387** of 600;
differential is 86 of 150 and has not moved. `examples/collect.mwl`'s frontier is unchanged —
`Core\Uri::parseQuery` at `collect.mwl:36`, then `Core\Csv::parse`, `Core\Validate::isEmail`,
`Core\Out::capture`.

## Next group — the rest of § 1 that needs no new dependency

Three slices, all in one file, each striking two lines from the outstanding list. `fold` and
`normalize` are deliberately **not** here: they need a Unicode-normalization dependency picked under
ADR 0051 § 4, which is its own group.

**Shared file set:** `crates/mwl-stdlib/src/str.rs` (`:49` `CLASS`'s rows — append after `:162`
`lines`, before `:169` `replace`; `:388` `address()`; `:1486` the test module),
`crates/mwl-stdlib/src/granularity.rs` (`:60` `Unit::CodePoint`, `:116` `pieces`),
`crates/mwl-stdlib/tests/spec-members-outstanding.txt`, and `tests/conformance/core/`.

- [ ] **`Core\Str::graphemes` and `codePoints`** (`01-core-library.md` § 1 *Extraction*, `:132`-`:133`).
      `graphemes(string $s): array<string>` and `codePoints(string $s): array<uint>` are
      `Unit::Grapheme.pieces()` and `Unit::CodePoint.pieces()` written out — `granularity.rs` already
      holds both, so this is two registry rows, two helper bodies and one case. Strike `§1 graphemes`
      and `§1 codePoints`.
- [ ] **`Core\Str::fromCodePoint` and `fromCodePoints`** (§ 1 *Transformation*, `:167`-`:168`). The
      inverse of `codePoints`, so write it in the same session while its case is open: a scalar value
      outside Unicode or in the surrogate range **throws** (R4), which is where it parts company with
      PHP's `chr`. Strike `§1 fromCodePoint` and `§1 fromCodePoints`.
- [ ] **`Core\Str::replaceAll` and `replaceRange`** (§ 1 *Transformation*, `:151`-`:152`).
      `replaceAll(string, array<string> $pairs, {caseInsensitive?})` is `strtr`'s longest-match-first
      single pass, **not** repeated `replace` — say so in the doc comment, because the difference is
      observable. `replaceRange` is `substr_replace` over `granularity::DEFAULT`, sharing
      `mwl_core_str_slice`'s offset/length rules at `str.rs:169`'s neighbours.

## Backlog

- § 1's `fold`, `normalize` and `compare` — a normalization dependency and a natural-order comparator,
  each picked under [ADR 0051](../adr/0051-standard-library-tiers.md) § 4 (`spec-members-outstanding.txt`).
- § 2 `Core\Arr` is 19 of the 44 outstanding keys, and ADR 0069's combination members are most of them.
- `mixed as bytes` has no lowering row and panics in `Lowering::convert`
  (`crates/mwl-ir/src/lower/expr.rs:842`); it needs a `Helper::TaggedToBytes`, and `unpack`'s `a`/`A`/`Z`
  fields stay pinned in `mwl-stdlib`'s unit tests until it exists.
- § 12's `Uri::parse`/`isValid` still owe the RFC 3986 dependency; `Csv` owes one too (plan, `Open now`).
- `do`/`while` is the one M4 control-flow statement that does not lower (plan, `Open now`).
- `docs/spec/02-php-migration.md` is 31% classified — `python tools/check-migration.py`.
