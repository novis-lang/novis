---
looper: 1
title: Strings with holes, html templates and Str::format write every piece into the result
summary: "Reached when an interpolated string, an html template and a literal Core\\Str::format call allocate the result once, writing escaped text, numbers and placeholders straight into it."
context: [crates/nvs-runtime/src/string.rs, crates/nvs-ir/src/lower/expr.rs, docs/rules/types/string-holes.md, docs/rules/core-classes/html-template.md]
stages:
  - id: 1
    title: "the runtime joins typed parts in one allocation: strings, integers and html-escaped text"
    status: done
    context: [crates/nvs-render/src/html.rs, crates/nvs-runtime/src/helpers.rs, docs/rules/core-classes/html-auto-escape.md]
  - id: 2
    title: "interpolated strings and html templates lower their holes to typed parts"
    status: done
    context: [crates/nvs-ir/src/ir.rs, crates/nvs-codegen/src/emit.rs, crates/nvs-codegen/src/lib.rs]
  - id: 3
    title: "a literal Core\\Str::format writes each placeholder into the result"
    status: done
    context: [crates/nvs-stdlib/src/format.rs]
  - id: 4
    title: "the changed features are measured again, and their allocation counts are declared"
    status: open
    context: [benches/members/README.md, benches/compare/profile.sh]
  - id: 5
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: parts-join
    stage: 1
    name: typed parts join in one allocation, and an escaped part writes what the HTML sink's escape writes
    run: [cargo, test, --lib, "string::"]
    timeout: 1800
    tests:
      - typed_parts_join_in_one_allocation
      - an_integer_part_writes_the_digits_int_to_string_writes
      - an_escaped_part_writes_what_the_html_sink_escape_writes
      - a_join_past_the_memory_limit_is_refused_before_a_byte_is_copied
---

# Strings with holes, html templates and Str::format write every piece into the result

## Why

A profile of `nvs serve` on one pinned core (`bun nv bench-compare --profile html-page`, 2026-11-05 at
`293a3b9b9`) served the `html-page` case at 10,551 requests a second, 94.8 µs each. The case builds 100
table rows from html templates with three holes each, and uses two `Core\Str::format` calls per row. Of
the serving core:

- Escaping holes took 11.4% (`nvs_core_html_escape_text`, `escaped_text`, `nvs_render::html::escape`).
- `Core\Str::format`'s placeholders took 9.4% (`nvs_core_str_format_piece`, `format::format`).
- Joining took 7.4% (`nvs_str_concat_n`, `concat_onto`).
- Integers turned into text took 6.2% (`nvs_int_to_string`, `decimal_digits`).
- The allocator's per-request accounting took about 12% (`Accounting::alloc`, `dealloc`, `realloc`).

All of it is one pattern. Each hole becomes a temporary string first: an escaped copy of a name with an
`&` in it, the digits of an integer, a rendered placeholder. Then `nvs_str_concat_n` copies every
temporary into the result and frees it. The digits themselves are cheap: `decimal_digits`
(`crates/nvs-runtime/src/helpers.rs:63`) costs its allocation, not its loop. A literal
`Core\Str::format` template is already split at compile time (`FormatPiece`). Each placeholder still
parses its spec, such as `%02d`, and allocates its own string on every call
(`crates/nvs-stdlib/src/format.rs:142`).

Writing each part straight into the result removes one allocation, one copy and one free per hole, and
the accounting for each of them. Every program benefits: every interpolated string with a number in it,
every html template, and every literal `Core\Str::format` call.

## Standing decisions

1. **One runtime entry joins typed parts.** `nvs_str_concat_parts` takes an array of parts. Each part is
   a kind and a payload: `Str` (a string, copied as it is), `Int` (an `i64`, written as decimal digits),
   `HtmlText` (a string, escaped as it is copied) and `Format` (a value and a pre-parsed placeholder,
   stage 3). It makes two passes: it measures every part, asks the budget once for the whole result,
   then writes every part. `nvs_str_concat_n` stays for joins of strings only, and its in-place append
   onto a solely-owned left operand stays (`crates/nvs-runtime/src/string.rs:1362`).
2. **The escape keeps one table.** `nvs-render` gains `escaped_len(&str)` and `escape_into(&str, &mut
   out)`, and `escape` is rebuilt on the two (`crates/nvs-render/src/html.rs:168`). The `&`, `<`, `>`,
   `"` and `'` table and the replacement of unterminated bidirectional controls stay in that one place
   (`rule:core-classes/html-auto-escape`). The escaped text written into a template is byte for byte
   what `escape` returns today.
3. **Taint, `secret` and `Markup` rules do not change.** The checker decides which holes are escaped,
   which are spliced raw (`Markup`), and which do not compile, exactly as today
   (`rule:core-classes/html-template`). Only the lowering of an escaped hole changes, from a call that
   returns a string to an `HtmlText` part.
4. **`Format` parts are pre-parsed at compile time.** The checker already records each placeholder's
   text (`FormatPiece::Converted`). The lowering parses it once and writes the spec as a constant, and the part writer runs `Spec`'s own conversion and
   padding (`crates/nvs-stdlib/src/format.rs:450`). While it measures, a placeholder renders into a
   64-byte buffer on the stack. A longer one, and any conversion the part writer does not handle, keeps
   today's `nvs_core_str_format_piece` and joins as a `Str` part. So no placeholder changes what it
   prints or which error it throws.
5. **Floats and every other conversion stay as they are.** A hole that is not an `int`, a string or a
   `Format` placeholder is converted as today and joins as a `Str` part.
6. **`echo` of an html template is untouched.** It already writes each part to the output with no join
   (`echo_html_template_parts`, `crates/nvs-ir/src/lower/expr.rs:848`).
7. **Out of scope:** making `Markup` a rope, and changing `Core\Html::join`. A row-by-row page is
   already one allocation per row after this goal, and `Markup + Markup` already appends in place.
8. **Measured by counts.** The evidence is the member ledger's allocations per operation and the
   profile's shares, before and after, in the commit messages.
9. **Lanes.** Stage 1 is the runtime and `nvs-render`. Stage 2 is `nvs-ir` and `nvs-codegen`, which
   stage 3 also touches, so stages 1 to 3 run in order. The goal shares no file with `json-one-pass` or
   `ceiling-wake`, so it may run as a side goal beside either.
10. **The runtime owns an integer placeholder, and the checker packs it.** `nvs-runtime` cannot depend
   on `nvs-stdlib`, so the integer half of `Spec`'s conversion and padding moved down into
   `nvs_runtime::placeholder::IntPlaceholder`, and `nvs_stdlib::format`'s `render` writes every integer
   placeholder through it. One home, so the two paths cannot print different text. The checker calls
   `nvs_stdlib::format::integer_part` once per placeholder and records `FormatPiece::Integer` with the
   packed `StrPart::FORMAT` kind word. The kind is the word's low byte, and the placeholder is the
   other 56 bits, so a part stays two words and the join parses nothing. A width or precision past
   `PACKED_MAX` (16383) does not pack, and the checker already stops at four digits. `%s` over a
   `string` with a flag, a width or a precision stays a `Converted` piece through
   `nvs_core_str_format_piece`, because its width counts graphemes, which `nvs_stdlib::granularity`
   owns. Floats stay with decision 5. Memory: none, the digits are written on the stack.

## Stages

### 1. the runtime joins typed parts in one allocation: strings, integers and html-escaped text

**Does:** the keystone. Nothing calls it yet, so the tests the `parts-join` check names are its proof.

- [x] Add `escaped_len` and `escape_into`, and rebuild `escape` on them. `rule:core-classes/html-auto-escape`
  `crates/nvs-render/src/html.rs:168`
- [x] Add a digit writer that writes into a buffer, beside `decimal_digits`, and rebuild
  `decimal_digits` on it. `crates/nvs-runtime/src/helpers.rs:63`
- [x] Add `nvs_str_concat_parts` and its part type, with the `Str`, `Int` and `HtmlText` kinds, next to
  `nvs_str_concat_n`, and register its symbol. `crates/nvs-runtime/src/string.rs:1532`
  `crates/nvs-runtime/src/helpers.rs:3626`
- [x] Write the four tests the `parts-join` check names. `crates/nvs-runtime/src/string.rs:1532`

### 2. interpolated strings and html templates lower their holes to typed parts

**Does:** every interpolated string and html template uses the new join.

**Split:** an `Explore` subagent lists every producer of `InstKind::Concat` and every reader of its
pieces in `nvs-ir`, `nvs-codegen` and the IR passes, as `file:line` with one line each.

- [x] Give `InstKind::Concat`'s pieces a kind, and keep every pass that reads the pieces right.
  `rule:types/string-holes` `crates/nvs-ir/src/ir.rs:1636`
- [x] Lower an `int` hole of an interpolated string to an `Int` part instead of converting it first.
  `crates/nvs-ir/src/lower/expr.rs:1014` `crates/nvs-ir/src/lower/expr.rs:2272`
- [x] Lower an escaped html template hole to an `HtmlText` part. `rule:core-classes/html-template`
  `crates/nvs-ir/src/lower/expr.rs:2585` `crates/nvs-ir/src/lower/expr.rs:2675`
- [x] Emit `nvs_str_concat_parts` for a join with any part that is not a string.
  `crates/nvs-codegen/src/emit.rs:3953` `crates/nvs-codegen/src/lib.rs:2086`

### 3. a literal Core\Str::format writes each placeholder into the result

**Does:** removes the per-placeholder parse and allocation.

- [x] Add the `Format` part, which writes through `Spec`'s conversion and padding.
  `crates/nvs-stdlib/src/format.rs:450` `crates/nvs-stdlib/src/format.rs:699`
  - [x] `IntPlaceholder` in `nvs-runtime`, packed into the kind word, and `StrPart::FORMAT`.
  - [x] `format`'s `render` writes integer placeholders through it, and `Digits` is gone.
  - [x] A test that each packed part writes what `format` writes.
- [x] Lower a `FormatPiece::Converted` placeholder to a `Format` part with its pre-parsed spec, and keep
  decision 4's fallback. `crates/nvs-ir/src/lower/expr.rs:2512` `crates/nvs-ir/src/lower/expr.rs:2384`
  - [x] The checker records `FormatPiece::Integer` with the packed word.
  - [x] `nvs-ir` lowers it to `PartKind::Format`, and a typed piece alone is still joined.
  - [x] Codegen stores the word as the part's kind.
- [x] Rewrite the doc of `PIECE_SYMBOL` and `nvs_core_str_format_piece` as a whole, saying which
  placeholders still reach them. `crates/nvs-stdlib/src/format.rs:125`

### 4. the changed features are measured again, and their allocation counts are declared

**Does:** records what the goal bought, in counts, and makes the counts a bound.

- [ ] An `Explore` subagent lists the bench of each changed feature: html templates, strings with
  holes, `.` and `+` joins, and `Core\Str::format`. Where html templates with holes have no bench, add
  one at the path `bun nv proofs --id '<feature>'` prints. `benches/members/README.md:1`
- [ ] Run `bun nv proofs --record-perf --force --id '<feature>'` for each, and for every other figure
  `bun nv proofs --gate` then names as stale. Today `Core\Str::format` makes 2 allocations per call and
  `lang:expressions/string-operators` 4.
- [ ] Add a `// bench: allocations N` line to each of those benches, with the new count.
- [ ] Run `bun nv bench-compare --profile html-page`, and put its shares and counts beside the ones in
  *Why* in the commit message. `benches/compare/profile.sh:1`

### 5. the goal is verified once, and every failure is fixed

- [ ] Run `looper verify` over the whole goal.
- [ ] Fix every failure it names in one pass, never deleting or loosening a test.
- [ ] Run `looper verify` once more, and it is green.

## Handoff

### State

Stages 1 to 3 are done, and `parts-join` is green. *Why*'s figures are from `293a3b9b9`.

- Each `Concat` piece is a `ConcatPart`: a `PartKind` (`Str`, `Int`, `HtmlText`, `Format(word)`) and
  a value. A join of strings alone consumes its leading piece. A typed join consumes none
  (`Lowering::emit_concat`, `ConcatPart::all_str`). A typed piece alone is still joined
  (`join_pieces`), so `Core\Str::format("%04d", $n)` is one `concat format 0x… vN`.
- Decision 10 is how a placeholder is written: `IntPlaceholder` (`crates/nvs-runtime/src/placeholder.rs`),
  `integer_part` (`crates/nvs-stdlib/src/format.rs`), `FormatPiece::Integer`
  (`crates/nvs-types/src/expr_table.rs`). `a_packed_integer_part_writes_what_format_writes` and
  `a_placeholder_packs_into_a_kind_word_and_back` pass.
- A smoke run printed `[-0042] [ff    ] [*****255] [+7]`, `0009` and `ab      |   12|`.
- `env-secret-proofs` is the driver's stale state for a retired check: `looper check --name
  env-secret-proofs` says no check matches, and nothing in the tree names it.

Expected red in stage 5:
- Each `nvs-ir` snapshot of an interpolation with an `int` hole or a template with a hole. It now
  prints `int vN` and `html vN` pieces and releases the leading literal after the join.
- Each `nvs-ir` snapshot of a literal `Core\Str::format` with an integer placeholder that has a flag,
  a width or a non-plain conversion. It now prints `format 0x… vN` where it called
  `nvs_core_str_format_piece`.
- `INSTA_UPDATE=always bun nv verify -p nvs-ir` rewrites them; check each diff for that shape.

### Next

**Stage 4**, files `benches/members/`, `benches/compare/profile.sh` and the perf ledgers. No code.
Follow the stage's items in order: the `Explore` subagent lists the benches first.
`Core\Str::format`'s count was 2 allocations per call. With an integer placeholder it should now be
1. A `%-10s` over a string still makes 2.

### Backlog
