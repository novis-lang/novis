---
looper: 1
title: Strings with holes, html templates and Str::format write every piece into the result
summary: "Reached when an interpolated string, an html template and a literal Core\\Str::format call allocate the result once, writing escaped text, numbers and placeholders straight into it."
context: [crates/nvs-runtime/src/string.rs, crates/nvs-ir/src/lower/expr.rs, docs/rules/types/string-holes.md, docs/rules/core-classes/html-template.md]
stages:
  - id: 1
    title: "the runtime joins typed parts in one allocation: strings, integers and html-escaped text"
    status: open
    context: [crates/nvs-render/src/html.rs, crates/nvs-runtime/src/helpers.rs, docs/rules/core-classes/html-auto-escape.md]
  - id: 2
    title: "interpolated strings and html templates lower their holes to typed parts"
    status: open
    context: [crates/nvs-ir/src/ir.rs, crates/nvs-codegen/src/emit.rs, crates/nvs-codegen/src/lib.rs]
  - id: 3
    title: "a literal Core\\Str::format writes each placeholder into the result"
    status: open
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

## Stages

### 1. the runtime joins typed parts in one allocation: strings, integers and html-escaped text

**Does:** the keystone. Nothing calls it yet, so the tests the `parts-join` check names are its proof.

- [ ] Add `escaped_len` and `escape_into`, and rebuild `escape` on them. `rule:core-classes/html-auto-escape`
  `crates/nvs-render/src/html.rs:168`
- [ ] Add a digit writer that writes into a buffer, beside `decimal_digits`, and rebuild
  `decimal_digits` on it. `crates/nvs-runtime/src/helpers.rs:63`
- [ ] Add `nvs_str_concat_parts` and its part type, with the `Str`, `Int` and `HtmlText` kinds, next to
  `nvs_str_concat_n`, and register its symbol. `crates/nvs-runtime/src/string.rs:1532`
  `crates/nvs-runtime/src/helpers.rs:3626`
- [ ] Write the four tests the `parts-join` check names. `crates/nvs-runtime/src/string.rs:1532`

### 2. interpolated strings and html templates lower their holes to typed parts

**Does:** every interpolated string and html template uses the new join.

**Split:** an `Explore` subagent lists every producer of `InstKind::Concat` and every reader of its
pieces in `nvs-ir`, `nvs-codegen` and the IR passes, as `file:line` with one line each.

- [ ] Give `InstKind::Concat`'s pieces a kind, and keep every pass that reads the pieces right.
  `rule:types/string-holes` `crates/nvs-ir/src/ir.rs:1636`
- [ ] Lower an `int` hole of an interpolated string to an `Int` part instead of converting it first.
  `crates/nvs-ir/src/lower/expr.rs:1014` `crates/nvs-ir/src/lower/expr.rs:2272`
- [ ] Lower an escaped html template hole to an `HtmlText` part. `rule:core-classes/html-template`
  `crates/nvs-ir/src/lower/expr.rs:2585` `crates/nvs-ir/src/lower/expr.rs:2675`
- [ ] Emit `nvs_str_concat_parts` for a join with any part that is not a string.
  `crates/nvs-codegen/src/emit.rs:3953` `crates/nvs-codegen/src/lib.rs:2086`

### 3. a literal Core\Str::format writes each placeholder into the result

**Does:** removes the per-placeholder parse and allocation.

- [ ] Add the `Format` part, which writes through `Spec`'s conversion and padding.
  `crates/nvs-stdlib/src/format.rs:450` `crates/nvs-stdlib/src/format.rs:699`
- [ ] Lower a `FormatPiece::Converted` placeholder to a `Format` part with its pre-parsed spec, and keep
  decision 4's fallback. `crates/nvs-ir/src/lower/expr.rs:2512` `crates/nvs-ir/src/lower/expr.rs:2384`
- [ ] Rewrite the doc of `PIECE_SYMBOL` and `nvs_core_str_format_piece` as a whole, saying which
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

Nothing is built yet. The figures in *Why* were taken on 2026-11-05 at `293a3b9b9`.

### Next

**Stage 1**, files `crates/nvs-render/src/html.rs`, `crates/nvs-runtime/src/helpers.rs` and
`crates/nvs-runtime/src/string.rs`. No gate.

- [ ] `escaped_len` and `escape_into`. `rule:core-classes/html-auto-escape` `crates/nvs-render/src/html.rs:168`
- [ ] The digit writer. `crates/nvs-runtime/src/helpers.rs:63`
- [ ] `nvs_str_concat_parts` and its symbol. `crates/nvs-runtime/src/string.rs:1532`
  `crates/nvs-runtime/src/helpers.rs:3626`
- [ ] The four tests. `crates/nvs-runtime/src/string.rs:1532`

### Backlog
