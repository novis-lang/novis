---
# GENERATED FILE — written by website/scripts/sync-rules.mjs from docs/rules/. Do not edit.
title: "Text, bytes and literal types"
description: "A string is UTF-8 for its whole lifetime, bytes is its unencoded peer, and a literal can be a type of its own."
editUrl: false
lastUpdated: false
tableOfContents: false
prev:
  link: /docs/rules/types/declarations-and-numbers/
  label: "Declarations and numbers"
next:
  link: /docs/rules/types/arrays-and-property-keys/
  label: "Arrays and property keys"
---

<p class="nv-section-lead">A string is UTF-8 for its whole lifetime, bytes is its unencoded peer, and a literal can be a type of its own.</p>

<div class="nv-counts"><div class="nv-count" data-kind="total"><span class="nv-count-value">7</span><span class="nv-count-label">rules</span></div><div class="nv-count" data-kind="shipped"><span class="nv-count-value">7</span><span class="nv-count-label">shipped</span></div><div class="nv-count" data-kind="designed"><span class="nv-count-value">0</span><span class="nv-count-label">designed</span></div><div class="nv-count" data-kind="php"><span class="nv-count-value">3</span><span class="nv-count-label">differ from PHP</span></div></div>

<ol class="nv-rule-list"><li><a href="#ordering">Only the types the table orders may be ordered, and two objects need <code>Comparable</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#string-is-utf8">A <code>string</code> is valid UTF-8 for its whole lifetime, and its unmarked unit is the grapheme cluster</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#bytes"><code>bytes</code> is a primitive peer to <code>string</code> for data that carries no encoding</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span><span class="nv-rule-list-flag" title="Differs from PHP">PHP</span></li><li><a href="#duration-literal"><code>1h30m</code> is a <code>Core\Time\Duration</code> constant, in one grammar shared by source, <code>parse</code> and <code>nvs.toml</code></a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#literal-types">A <code>string</code> or <code>int</code> literal is its own type, and a union of them is a closed set</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#constant-in-type-position">A scalar class constant used as a type folds to its own literal type</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li><li><a href="#enum-case-type">An enum case used as a type is a narrowed subtype of its enum, never its backing integer</a><span class="nv-rule-list-status" data-status="shipped">Shipped</span></li></ol>

<div class="nv-rule" id="ordering">

## Only the types the table orders may be ordered, and two objects need `Comparable`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#ordering"><code>types/ordering</code></a>
</div>

`< <= > >= <=>` are defined on a **closed** list of operand pairs, and refused everywhere else:

| operands | result |
|---|---|
| two numerics, `int` against `uint` included | `bool` (`int` for `<=>`), mathematically exact over the full range of both |
| `decimal` against `int`, `uint` or `float` | `bool`, exact — a comparison is computable where a common arithmetic type is not |
| two `bool`s | `false < true`, the ordering of the one bit they already are |
| two objects whose static type is provably the same class implementing `Comparable` | `bool`/`int`, via `compareTo`; a throwing `compareTo` propagates as a checked status like any other call |
| any other operands | **compile error** |

There is no fallback. Everything else PHP orders, it orders by converting an operand first, and there
is no implicit conversion for that to be. So two strings order through `Core\Str::compare`, an enum
case orders through its backing `as int` ([`enums/closed-integer-type`](/docs/rules/enums/#closed-integer-type "An enum declares a new, closed, named integer type")), and an `array<T>`, a
`callable` and `null` do not order at all. Two objects with no `Comparable` between them are a
compile error whose diagnostic names `Comparable` as the fix, however the receiver was spelled — an
erased `object` and a shape type included.

An operand whose static type names no row is answered from its runtime tag, and refuses as a
*catchable throw* where the tags name none. Two consequences follow from the tag being all there is:
two objects behind two `mixed`s throw, because `compareTo` is dispatched from the class the *site*
named, and an enum case orders as the integer it is even though the written spelling is still refused
— which is where the author is told to say `as int`.

`==` and `!=` are a different question and are unaffected by any of this.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>There is no fallback ordering — PHP orders anything by converting an operand first, and Novis refuses a <code>string</code>, <code>bytes</code>, <code>array&lt;T&gt;</code>, <code>callable</code>, enum case or <code>null</code> operand where it is written</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#arithmetic" title="An arithmetic operator answers in its operands' own type, and overflow throws rather than wrapping or promoting"><code>types/arithmetic</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/unions-and-conversion/#unions-and-mixed" title="A union permits only what every member permits, and mixed is the one position checked nowhere"><code>types/unions-and-mixed</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0013.md">record 0013</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0010.md">record 0010</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0047.md">record 0047</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-ordering-over-a-mixed-operand-is-decided-by-its-tag.nvst"><code>tests/conformance/lang/an-ordering-over-a-mixed-operand-is-decided-by-its-tag.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-operator-agrees-whether-its-operand-is-typed-or-tagged.nvst"><code>tests/conformance/lang/an-operator-agrees-whether-its-operand-is-typed-or-tagged.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/comparable_and_stringable.rs"><code>crates/nvs-types/tests/comparable_and_stringable.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="string-is-utf8">

## A `string` is valid UTF-8 for its whole lifetime, and its unmarked unit is the grapheme cluster

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#string-is-utf8"><code>types/string-is-utf8</code></a>
</div>

A `string` cannot hold invalid UTF-8 at any point in its lifetime. The invariant is enforced at every
construction site — literals, conversions, concatenation, and every stdlib member that builds a
string — the same way an array's element type is enforced on every write ([`types/arrays`](/docs/rules/types/arrays-and-property-keys/#arrays "An array is PHP's ordered hash with string keys and a declared element type")). That
is what removes PHP's `mbstring` split: there is only one encoding a `string` can hold, so there is
only one correct answer to "how long is it".

**A `string`'s length, indexing and iteration operate on extended grapheme clusters** (Unicode UAX
#29) — the unit a person reading the source calls "one character", including a flag emoji, an emoji
built from a ZWJ sequence, or a letter with a combining accent. Byte-level and codepoint-level
operations remain available under separately named members, the inverse of PHP's default.
`nvs_stdlib::granularity` states that default in code, once, and every `Core\Str` member with a unit
reads it from there.

Two consequences an implementer owes: what counts as one character is pinned to whichever Unicode
version `nvs-runtime` embeds, and can change across a runtime upgrade; and `Core\Str::length` is O(n)
where PHP's `strlen` is O(1), a vectorized scan over ASCII and a full segmentation run over anything
else. Normalization (NFC/NFD) is explicitly out of scope — grapheme awareness says nothing about
whether two visually identical strings compare equal, exactly as PHP leaves it.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p><code>strlen</code> counts bytes and needs <code>mb_strlen</code> plus a correct ambient encoding to count characters; a Novis <code>string</code> has one encoding and one length, counting extended grapheme clusters unconditionally</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/text-and-literal-types/#bytes" title="bytes is a primitive peer to string for data that carries no encoding"><code>types/bytes</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0009.md">record 0009</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/str-length-counts-graphemes.nvst"><code>tests/conformance/core/str-length-counts-graphemes.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/str-graphemes-partitions-the-subject-the-other-two-members-measure.nvst"><code>tests/conformance/core/str-graphemes-partitions-the-subject-the-other-two-members-measure.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/str-graphemes-and-code-points-count-different-units.nvst"><code>tests/conformance/core/str-graphemes-and-code-points-count-different-units.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-interpolated-string-keeps-its-grapheme-clusters.nvst"><code>tests/conformance/lang/an-interpolated-string-keeps-its-grapheme-clusters.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="bytes">

## `bytes` is a primitive peer to `string` for data that carries no encoding

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<span class="nv-rule-flag">Differs from PHP</span>
<a class="nv-rule-id" href="#bytes"><code>types/bytes</code></a>
</div>

`bytes` is a primitive scalar, peer to `string`, for data with no encoding at all — a file's contents,
a socket read, a hash digest, a request body before anyone has claimed it is text. It has the same
copy-on-write, interned-buffer value semantics `string` already has, minus the UTF-8 invariant, and it
is indexed and sliced by **byte offset**; there is no other unit for it to be ambiguous about.

It is not a class: a flat, contiguous, immutable-until-copied buffer has no use for identity,
properties or a vtable. It is not `array<uint>` either, which would cost a tagged value per byte plus
a per-write element check for a type whose whole point is that it has no per-element structure.

Anywhere the host hands a program data it has not itself asserted is text, it hands over `bytes`.
Structured input stays `array<mixed>` — `Core\Request`, `Core\Server` and `Core\Json::decode`'s result
are untyped deliberately ([`types/unions-and-mixed`](/docs/rules/types/unions-and-conversion/#unions-and-mixed "A union permits only what every member permits, and mixed is the one position checked nowhere")) — and this rule is about the *scalar* payload
underneath, once one is pulled out of `mixed`. Converting is `as`, checked in the direction that can
fail ([`types/conversion`](/docs/rules/types/unions-and-conversion/#conversion "expr as T is the only conversion, and it produces a T or throws")), which is what makes "treat these untrusted bytes as text" a reviewable,
throwing event rather than an unasserted assumption.

There is no dedicated literal token: `"…" as bytes` covers the valid-UTF-8 case for free, and
`Core\Bytes::fromHex()`/`::fromBase64()` cover arbitrary binary constants.

<aside class="nv-rule-diverges">
<p class="nv-rule-diverges-label">Where this differs from PHP</p>
<p>PHP has one type for text and for a buffer and never says which a value is; a request body, a socket read, a file's contents and a digest all arrive as <code>bytes</code> and become text only by a conversion that can throw</p>
</aside>

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/text-and-literal-types/#string-is-utf8" title="A string is valid UTF-8 for its whole lifetime, and its unmarked unit is the grapheme cluster"><code>types/string-is-utf8</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/declarations-and-numbers/#grammar" title="The type grammar is a closed set of atoms under unions and intersections"><code>types/grammar</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0009.md">record 0009</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0006.md">record 0006</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0012.md">record 0012</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/bytes-indexes-and-orders-by-byte-offset.nvst"><code>tests/conformance/core/bytes-indexes-and-orders-by-byte-offset.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/bytes-predicates-search-octets-where-core-str-searches-characters.nvst"><code>tests/conformance/core/bytes-predicates-search-octets-where-core-str-searches-characters.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/string-and-bytes-convert-both-ways.nvst"><code>tests/conformance/lang/string-and-bytes-convert-both-ways.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-bytes-in-a-condition-is-falsy-only-when-empty.nvst"><code>tests/conformance/lang/a-bytes-in-a-condition-is-falsy-only-when-empty.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/a-body-stream-chunk-is-tainted-and-a-plain-bytes-binding-refuses-it.nvst"><code>tests/conformance/core/a-body-stream-chunk-is-tainted-and-a-plain-bytes-binding-refuses-it.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="duration-literal">

## `1h30m` is a `Core\Time\Duration` constant, in one grammar shared by source, `parse` and `nvs.toml`

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#duration-literal"><code>types/duration-literal</code></a>
</div>

```
duration := ( DEC_INT unit )+
unit     := ns | us | ms | s | m | h | d | w
```

- **One token.** `1h30m` lexes as a single duration literal, maximal munch, not as three tokens.
- **Units strictly descend and may not repeat.** `1h30m` is accepted; `30m1h` and `1h1h` are lexer
  errors naming this rule, so there is exactly one spelling of any given constant.
- **Only after a plain decimal integer** — never after `0x…`, `0b…`, a float or an exponent, so `0x1d`
  stays a hex literal and `1.5s` is an error rather than a rounded duration.
- **Lower case only**; `30S` is a diagnostic, not a second spelling.
- **No sign.** `-7d` does not parse; a backwards step is `->minus(7d)`.
- `d` is exactly 24 h and `w` exactly 168 h. A *calendar* day is a `DateTime` unit and never a
  `Duration` at all.

The type is `Core\Time\Duration`, always — the suffix *is* the type, with nothing
untyped-until-placed about it ([`types/numeric-literal-placement`](/docs/rules/types/declarations-and-numbers/#numeric-literal-placement "A numeric literal is untyped until it is placed, and as T is a placing position")). A literal is a compile-time
constant folded to a single nanosecond count and emitted into the constant pool, so `{timeout: 30s}`
allocates nothing at run time and a literal beyond `Duration`'s range is a compile error, not a wrap.

`1h + 30m` does not compile — there is no operator overloading; write `1h30m` or `$a->plus($b)`. So
does `1h30m as int`; write `->toSeconds()`. `$n s` is not a literal; a computed count is
`Duration::seconds($n)`.

**One grammar, three places, one parser**: source, `Duration::parse($s)` at run time, and a `"30s"` in
`nvs.toml` at boot. `Duration`'s string form emits this grammar too, so a value round-trips through
`parse` over exactly the durations the grammar can spell — the non-negative ones. A negative duration
renders `-1h30m` for a reader, and `parse` refuses that leading `-` **by name** rather than reading a
positive value out of it.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/declarations-and-numbers/#numeric-literal-placement" title="A numeric literal is untyped until it is placed, and as T is a placing position"><code>types/numeric-literal-placement</code></a> <a href="/docs/rules/types/declarations-and-numbers/#integer-literals" title="An integer literal is decimal, 0x, 0o or 0b, and a leading zero is not a radix"><code>types/integer-literals</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0070.md">record 0070</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0046.md">record 0046</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0057.md">record 0057</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0062.md">record 0062</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0039.md">record 0039</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/duration-literal-and-parse-share-one-grammar.nvst"><code>tests/conformance/lang/duration-literal-and-parse-share-one-grammar.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-duration-literal-follows-one-grammar.nvst"><code>tests/conformance/reject/a-duration-literal-follows-one-grammar.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/reject/a-duration-unit-is-lower-case-only.nvst"><code>tests/conformance/reject/a-duration-unit-is-lower-case-only.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/core/time-duration-round-trips-through-parse-exactly-where-the-grammar-has-a-spelling.nvst"><code>tests/conformance/core/time-duration-round-trips-through-parse-exactly-where-the-grammar-has-a-spelling.nvst</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="literal-types">

## A `string` or `int` literal is its own type, and a union of them is a closed set

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#literal-types"><code>types/literal-types</code></a>
</div>

A `string` literal and an `int` literal are each their own type — the singleton type inhabited by that
exact value — parsed only in type position, the way `array<T>` is. `"a"|"b"|"c"` and `1|2|3` are
ordinary unions of those atoms, canonicalised like any other, and `?"a"` is sugar for `"a"|null`. They
are usable at every binding site ([`types/declaration`](/docs/rules/types/declarations-and-numbers/#declaration "Every binding declares its type, and no binding's type ever changes")), with no special case.

```php
function setMode("a"|"b"|"c" $mode) { … }   // the set is the type
```

Assignability and conversion:

| direction | behaviour |
|---|---|
| a literal type → its base type, and a literal union → its base type | **total, free** — a strict widening, the same representation |
| base type or `mixed` → a literal or literal-union type | **checked.** Throws unless the value equals one of the named literals |
| a wider literal union → a narrower one | needs a guard or a checked `as` — ordinary narrowing ([`types/narrowing`](/docs/rules/types/unions-and-conversion/#narrowing "Narrowing is flow-sensitive and branch-local, and there are exactly four spellings of it")) |

**Zero additional runtime representation.** A literal type shares its base type's tag and payload
exactly; the singleton-ness is enforced by the checker wherever the static type is known. The only
place it costs anything is where a value arrives through `mixed` or an isolate boundary, and the
checked conversion runs a membership test against the small, closed, compile-time-known set.

A failed conversion names the accepted set, generated from the type: ``` `"z"` is not one of `"a"`,
`"b"`, `"c"` ``` (`E0469`). That is a **compile** error where the operand settles the question by
itself — the target a closed set, the operand naming one value — and otherwise the ordinary checked
conversion answered at run time. A `tainted` or `secret` value needs the same laundering it would need
to leave `mixed` for any other typed binding; neither qualifier gets a rule of its own here.

Two limits are deliberate: **no `float` literal type**, because float equality is imprecise enough
that a singleton `0.1` is a footgun; and **no wildcard matching** over constant or case names, since
the whole point is that the accepted set is spelled out.

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/text-and-literal-types/#constant-in-type-position" title="A scalar class constant used as a type folds to its own literal type"><code>types/constant-in-type-position</code></a> <a href="/docs/rules/types/text-and-literal-types/#enum-case-type" title="An enum case used as a type is a narrowed subtype of its enum, never its backing integer"><code>types/enum-case-type</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/unions-and-conversion/#narrowing" title="Narrowing is flow-sensitive and branch-local, and there are exactly four spellings of it"><code>types/narrowing</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0047.md">record 0047</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0024.md">record 0024</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0033.md">record 0033</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0066.md">record 0066</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-conversion-into-a-closed-set-of-literals-is-checked-at-run-time.nvst"><code>tests/conformance/lang/a-conversion-into-a-closed-set-of-literals-is-checked-at-run-time.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-conversion-into-a-set-of-mixed-literal-kinds-tags-its-operand.nvst"><code>tests/conformance/lang/a-conversion-into-a-set-of-mixed-literal-kinds-tags-its-operand.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-conversion-the-operand-disproves-is-a-compile-error.nvst"><code>tests/conformance/lang/a-conversion-the-operand-disproves-is-a-compile-error.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-true-or-false-target-is-a-literal-type-of-bool.nvst"><code>tests/conformance/lang/a-true-or-false-target-is-a-literal-type-of-bool.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/literal_types.rs"><code>crates/nvs-types/tests/literal_types.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="constant-in-type-position">

## A scalar class constant used as a type folds to its own literal type

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#constant-in-type-position"><code>types/constant-in-type-position</code></a>
</div>

`ClassName::CONST_NAME`, written where a type is expected, resolves at compile time to the constant's
own value — exactly as long as that value is a `string` or `int` compile-time constant.

```php
class Foo {
    public const string TYPE_A = "a";
    public const string TYPE_B = "b";
}

function handle(Foo::TYPE_A|Foo::TYPE_B $type) { … }   // exactly "a"|"b"
```

This is safe precisely because a scalar `const` is not a distinct nominal type: `Foo::TYPE_A`
genuinely *is* the string `"a"`, so folding it to that literal type changes nothing a caller could
observe — passing the bare `"a"` is exactly as valid.

A constant backed by a non-scalar type — an `array`, an object, a `float` — is **not eligible**, and
using one this way is a diagnostic naming the eligible types. An enum case is not folded either, and
for the opposite reason: it carries its enum's nominal type and stays a narrowed view of it
([`types/enum-case-type`](/docs/rules/types/text-and-literal-types/#enum-case-type "An enum case used as a type is a narrowed subtype of its enum, never its backing integer")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/text-and-literal-types/#literal-types" title="A string or int literal is its own type, and a union of them is a closed set"><code>types/literal-types</code></a> <a href="/docs/rules/types/text-and-literal-types/#enum-case-type" title="An enum case used as a type is a narrowed subtype of its enum, never its backing integer"><code>types/enum-case-type</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0047.md">record 0047</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0046.md">record 0046</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/literal_types.rs"><code>crates/nvs-types/tests/literal_types.rs</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-syntax/src/parser/tests/ty.rs"><code>crates/nvs-syntax/src/parser/tests/ty.rs</code></a></dd></div></dl>

</div>

<div class="nv-rule" id="enum-case-type">

## An enum case used as a type is a narrowed subtype of its enum, never its backing integer

<div class="nv-rule-tags">
<span class="nv-rule-status" data-status="shipped">Shipped</span>
<a class="nv-rule-id" href="#enum-case-type"><code>types/enum-case-type</code></a>
</div>

`EnumName::CaseName`, written where a type is expected, does **not** resolve to its backing integer. It
names a new checker-only type: a subtype of `EnumName` inhabited by exactly that one case.

```php
enum Mode { Read, Write, Admin }

function grant(Mode::Read|Mode::Write $m) { … }   // accepts only those two cases
```

Folding it to the cases' backing integers would let a caller satisfy the parameter with a bare `int`,
which is exactly the hole a checked `int → Mode` conversion closes
([`enums/closed-integer-type`](/docs/rules/enums/#closed-integer-type "An enum declares a new, closed, named integer type")). An enum-case type is therefore its own atom kind, never unified by
canonicalisation with an int literal type that happens to share a case's value, because the two carry
different runtime tags.

A case-subset union may name cases of more than one enum, or mix case atoms with unrelated atoms,
exactly as any other heterogeneous union may. Widening a case-subset union to its enum is total and
free; going the other way is checked and throws unless the value's case is one of the named ones — a
further-restricted form of the existing enum conversion, not a new kind
([`types/conversion`](/docs/rules/types/unions-and-conversion/#conversion "expr as T is the only conversion, and it produces a T or throws")). `E0470` names the accepted cases.

A binding is narrowed to a case-subset type **through `as` and nowhere else**: `$m == Mode::Read` does
not narrow `$m` in the branch it guards ([`types/narrowing`](/docs/rules/types/unions-and-conversion/#narrowing "Narrowing is flow-sensitive and branch-local, and there are exactly four spellings of it")). Like a literal type, this costs
nothing at runtime — it shares the enum's existing zero-byte representation
([`enums/representation`](/docs/rules/enums/#representation "An enum value costs nothing beyond the integer it is")).

<dl class="nv-rule-meta"><div class="nv-rule-meta-row"><dt>See also</dt><dd><a href="/docs/rules/types/text-and-literal-types/#literal-types" title="A string or int literal is its own type, and a union of them is a closed set"><code>types/literal-types</code></a> <a href="/docs/rules/types/unions-and-conversion/#conversion" title="expr as T is the only conversion, and it produces a T or throws"><code>types/conversion</code></a> <a href="/docs/rules/types/unions-and-conversion/#narrowing" title="Narrowing is flow-sensitive and branch-local, and there are exactly four spellings of it"><code>types/narrowing</code></a></dd></div><div class="nv-rule-meta-row"><dt>Decided in</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0047.md">record 0047</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0010.md">record 0010</a> <a href="https://github.com/novis-lang/novis/blob/main/docs/decisions/0007.md">record 0007</a></dd></div><div class="nv-rule-meta-row"><dt>Guarded by</dt><dd><a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/a-conversion-into-a-closed-set-of-enum-cases-is-checked-at-run-time.nvst"><code>tests/conformance/lang/a-conversion-into-a-closed-set-of-enum-cases-is-checked-at-run-time.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/tests/conformance/lang/an-enum-case-comparison-narrows-its-subject.nvst"><code>tests/conformance/lang/an-enum-case-comparison-narrows-its-subject.nvst</code></a> <a href="https://github.com/novis-lang/novis/blob/main/crates/nvs-types/tests/literal_types.rs"><code>crates/nvs-types/tests/literal_types.rs</code></a></dd></div></dl>

</div>
