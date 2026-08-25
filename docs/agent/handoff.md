# Handoff

## State

**Spec § 7 is complete.** `Core\Bytes` is all twelve members and `Core\Encoding` was already whole,
so nothing in that section is owed. `pack`/`unpack` share **one closed code table**, written once and
read twice; `crates/mwl-stdlib/src/bytes.rs`'s own module doc § *`pack`'s format is a closed grammar*
owns the table, the four decisions inside it, and the ten PHP codes it refuses with a replacement
named. Both are valgrind-clean, including the throwing path that releases a half-built array.

**Two divergences from PHP are deliberate and recorded there, not here** — `unpack` answers a
*positional* `array<mixed>` rather than PHP's name→value map (so `unpack(pack($f, ...$v), $f)` is
`$v` exactly, over one grammar), and octets the format does not describe **throw** rather than being
ignored.

**The one thing that blocks reading a buffer field from MWL**: `mixed as bytes` has no lowering row,
so `$fields[0] as bytes` **panics** in `Lowering::convert` (`crates/mwl-ir/src/lower/expr.rs:842`
names it, along with `array<T> as array<U>`). `unpack`'s `a`/`A`/`Z` fields are therefore pinned in
`mwl-stdlib`'s own unit tests and the conformance case observes only their boundary, as an element
count. It needs a `Helper::TaggedToBytes` beside `TaggedToInt`/`TaggedToUint`/`TaggedToFloat`.

Conformance is **386** of 600 (two new cases); differential is 86 of 150 and has not moved.
`examples/collect.mwl`'s frontier is unchanged — `Core\Uri::parseQuery` at `collect.mwl:36`, then
`Core\Csv::parse`, `Core\Validate::isEmail`, `Core\Out::capture`.

## Next group — the gate's own test, then § 1's splitters

The first item's *output* is the list the rest work from, and every item lives inside one crate.

**Shared file set:** `crates/mwl-stdlib/tests/` (`conformance_coverage.rs:52` is the shape to copy —
it already walks the registry the other way), `crates/mwl-stdlib/src/registry.rs:625` (`CLASSES`),
`crates/mwl-stdlib/src/str.rs` (`:49` `CLASS`'s rows, `:374` `address()`, `:1360` the test module —
append helpers before it), `crates/mwl-stdlib/src/granularity.rs` (`:53` `Unit`, `:114` `pieces`,
`:200` `byte_of_index`), `tests/conformance/core/`, and `docs/spec/01-core-library.md:101-169`.

- [ ] **`every_part_one_spec_member_is_registered`** (`docs/agent/loop-goal.md`'s acceptance list, and
      the plan's `Open now`). The loop's own definition of done and it does not exist: parse the
      `| \`name\` | \`sig\` |` rows out of `docs/spec/01-core-library.md` §§ 1-12, check each against
      `registry::CLASSES`, and fail naming the members that are missing. § 13 is out of scope
      (loop-goal § *Standing decisions*), so the walk stops at § 12's last table.
- [ ] **`Core\Str::chunk` and `Core\Str::lines`** (`01-core-library.md` § 1, `:130`-`:131`). Both split
      by a unit `crate::granularity` already segments — `chunk` by `DEFAULT`'s grapheme clusters, so
      it is `pieces` plus a counter, and `lines` splits on `\n` with a trailing `\r` dropped and no
      `PHP_EOL` anywhere. Neither takes a dependency.
- [ ] **`Core\Str::graphemes` and `codePoints`, with `fromCodePoint`/`fromCodePoints`** (§ 1, `:132`-
      `:133`, `:167`-`:168`). The two decoders are `granularity`'s segmentation and `char::len_utf8`
      read out as `array<string>`/`array<uint>`; the two constructors are their inverse and throw on a
      scalar that is not a code point (ADR 0063 R4, exactly as `Core\Bytes::fill` refuses a non-octet).

## Backlog

- `Helper::TaggedToBytes`, closing `mixed as bytes` — `mwl-ir`'s crate doc § *known gaps*.
- § 1's remaining five: `compare`, `replaceAll`, `replaceRange`, `fold`, `normalize` (the last owes a
  `NormalForm` enum and a Unicode normalization dependency under ADR 0051 § 4).
- ADR 0088's registry-wide qualifier classification; both `pack`/`unpack` formats and
  `Core\Str::format`'s template are sinks with nowhere yet to say so.
- ADR 0057's compile-time fold of a *literal* format/template, owed by three members now.
- § 12's `Core\Uri::parseQuery`, which is `examples/collect.mwl`'s first report.
- `do`/`while` is the one M4 control-flow statement that does not lower — `mwl-ir`'s crate doc.
