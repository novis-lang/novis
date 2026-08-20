# ADR 0009 — `string` is text; binary data is a distinct `bytes` type

- **Status:** Proposed — the default length/indexing granularity in *Decision § 2* depends on a cost
  measurement not yet taken. See *Revisiting*.
- **Date:** 2026-08-20
- **Scope:** the `string` and `bytes` primitive types; the UTF-8 invariant on `string`; the conversion
  between them; the default granularity of `string` length, indexing and iteration
- **Relates to:** [0002](0002-error-propagation.md) (a failed `bytes → string` conversion throws, so it
  propagates as a checked status like every other conversion in [0007](0007-explicit-type-system.md)),
  [0003](0003-extension-system.md) (a WIT byte buffer now has an exact MWL counterpart, and a WIT string
  the UTF-8 guarantee it always assumed), [0006](0006-isolated-script-execution.md) (a value of either type
  crossing an isolate boundary carries no encoding ambiguity with it — the same deep-copy-or-move rule
  applies to both), [0007](0007-explicit-type-system.md) (`string` is already an atom in its grammar; this
  ADR fixes what it *means* and adds `bytes` beside it as a new atom and a new conversion-table row)

> **In short:** PHP has one type for "a piece of text" and "a buffer of bytes," and it never says which one
> a given `string` is — that ambiguity is why PHP needs a whole parallel function set (`strlen` vs
> `mb_strlen`) and a global or per-call encoding to make either half work. MWL splits it: **`string` is
> guaranteed-valid UTF-8, always**, and **`bytes` is a new primitive, peer to `string`, for data with no
> encoding at all** — a file's contents, a socket read, a hash digest, a request body before anyone has
> claimed it is text. Conversion follows the rule every other conversion in
> [0007](0007-explicit-type-system.md) already follows: `string as bytes` is total and free (valid UTF-8 is
> already a valid byte sequence); `bytes as string` is checked and throws on invalid UTF-8. Where MWL is not
> yet settled: `string`'s default length/indexing/iteration should count **grapheme clusters** — what a
> person would call "one character," including a flag emoji or an accented letter built from combining
> marks — *provided that can be made cheap enough*; the named fallback if it cannot is byte length, which is
> what PHP gives you today. That fork is not decided here; see *Revisiting*.

## Context

PHP's `string` (`zend_string`) has always carried an explicit length beside its bytes, so it was
binary-safe from early on — no truncation at an embedded NUL, unlike a C `char*`. That was a real feature.
What it never gained is any notion of *encoding*. A `string` is a byte buffer; whether those bytes are
Latin-1, UTF-8, a JPEG, or a hash digest is something the programmer must remember and every function must
be told, because the type does not say. When the web moved to Unicode, PHP's answer was `mbstring` — a
second, optional function set (`mb_strlen`, `mb_substr`, `mb_strpos`, …) that only works correctly if the
encoding it assumes matches the bytes actually in the buffer, a fact tracked nowhere the type checker (such
as it is) can see. `strlen("café")` and `mb_strlen("café")` disagree, and nothing about the variable `$s`
tells you which answer the rest of the program expects.

This is not laziness so much as an artifact of when the decision was made: C had no vocabulary for
"encoded text" versus "bytes" in 1995, and neither did most of the software PHP was competing with. The
mistake was never revisiting it once Unicode text became the overwhelming common case on the web, and
instead leaving text-awareness as an opt-in extension layered on top of a type that still means "bytes."

For MWL this is not just a naming annoyance. [0007](0007-explicit-type-system.md) already treats "a
conversion of untrusted data should be a place in the source that can be reviewed" as a security argument
(priority 1), for exactly the same reason PHP's numeric coercions are a bug class. Silently treating an
attacker-controlled byte buffer as text — assuming an encoding nobody asserted — is the same shape of bug:
a length check computed on the wrong unit, a truncation that splits a multi-byte sequence, a comparison
that is exact in bytes but not in the characters a downstream system will decode. A type that forces the
question to be asked once, at a conversion site, is strictly better than a convention every function must
remember on its own.

## Decision

**`bytes` is a new primitive type, peer to `string`, for data with no encoding. `string` is guaranteed valid
UTF-8 for its entire lifetime. Converting between them follows [0007](0007-explicit-type-system.md)'s
existing rule: total in the safe direction, checked and throwing in the other.**

### 1. `bytes` joins the atom grammar

```
atom := … | 'bytes' | …
```

alongside `string` in [0007](0007-explicit-type-system.md) § 3. `bytes` is a primitive scalar with the same
copy-on-write, interned-buffer value semantics `string` already has — not a class, and not `array<uint>`:

- **Not a class.** A flat, contiguous, immutable-until-copied buffer has no use for object identity,
  properties, or a vtable. Making it a class would mean a second representation living beside `string`'s
  existing buffer machinery for no semantic gain, which is the "second arena setup" this codebase already
  treats as a bug rather than a variant ([0006](0006-isolated-script-execution.md)).
- **Not `array<uint>`.** An element-typed array costs a tagged value per element — 8 bytes per byte of
  actual data, plus every write going through the element-type check [0007](0007-explicit-type-system.md)
  § 5 requires. `bytes` reuses exactly the buffer `string` already needs, minus the UTF-8 invariant.

`bytes` is indexed and sliced by byte offset; there is no other unit to be ambiguous about, unlike `string`.

### 2. `string` is guaranteed valid UTF-8; default operations act on what a person calls "a character"

A `string` cannot hold invalid UTF-8 at any point in its lifetime — the invariant is enforced at every
construction site (literals, conversions, concatenation, stdlib functions that build strings), the same way
[0007](0007-explicit-type-system.md) § 5 enforces an array's element type on every write. This is what
actually removes the `mbstring` split: there is only one encoding a `string` can hold, so there is only one
correct answer to "how long is it."

The intended default: `string`'s length, indexing and iteration operate on **extended grapheme clusters**
(Unicode UAX #29) — the unit that matches what a person reading the source would call "one character,"
including a flag emoji, an emoji built from a ZWJ sequence, or a letter with a combining accent. Byte-level
and codepoint-level operations remain available, but under separate, explicitly-named functions — the
inverse of PHP's default, where byte semantics are the unmarked case and anything text-aware needs an
`mb_` prefix.

**This default is conditional, not unconditional**, on the cost being tractable — see *Revisiting* for what
"tractable" must be shown to mean before this ADR can move from Proposed to Accepted. The named fallback,
if grapheme-default proves too expensive, is byte length as the default — i.e., `string`'s basic length
function behaves the way PHP's `strlen` already does, and grapheme-awareness moves to its own explicitly
named function instead. Codepoint count (what Python 3 or Java effectively give you) is deliberately not the
fallback; see *Alternatives rejected*.

### 3. Conversion

| conversion | behaviour |
|---|---|
| `string as bytes` | **total, free.** Valid UTF-8 is already a valid byte sequence; no copy, no allocation — the same buffer, reinterpreted |
| `bytes as string` | **checked.** Validates the buffer is well-formed UTF-8; produces the `string` or throws. Never replaces, drops, or substitutes invalid bytes |

This is the same shape [0007](0007-explicit-type-system.md) § 2 already uses for every other conversion:
total where no information can be lost, checked and throwing everywhere else. No third option, no lossy
default — `iconv`'s `//IGNORE` and `//TRANSLIT` suffixes are exactly the silent-substitution failure mode
this ADR exists to avoid.

### 4. Where `bytes` shows up by default

Anywhere MWL currently hands the program data it has not itself asserted is text — a request body before
`Content-Type` has been consulted, a raw socket read, a file's contents, a hash or crypto digest — the host
should hand it over as `bytes`, not `string`. `Core\Request`, `Core\Server` (the [ADR 0012](0012-no-superglobals.md) replacements for `$_GET`/`$_POST`/`$_SERVER`) and `json_decode`'s result stay
`array<mixed>` exactly as [0007](0007-explicit-type-system.md) § 6 already decided (structured input is
untyped, deliberately); this ADR is about the *scalar* payloads underneath, once one is pulled out of
`mixed` — pulling a request body out as `bytes` and converting `as string` only when the program is willing
to assert the encoding is a security improvement in exactly the shape [0007](0007-explicit-type-system.md)
§ 6 already argues for `as uint`.

`bytes` literal syntax (a `b"…"` prefix, a constructor function, or something else) is not decided here —
per the plan's split, this ADR fixes the *semantics*, `docs/spec/00-overview.md` fixes the *spelling*.

## Consequences

**Positive**

- The `mbstring`-versus-plain-`string` split is gone, permanently, rather than papered over: there is one
  `string` type, and it has one length function with one answer.
- Treating untrusted bytes as text becomes a reviewable, explicit, throwing conversion — the same security
  argument [0007](0007-explicit-type-system.md) already made for numeric input, extended to encoding.
- No new operator and no new call-site shape: `bytes`/`string` conversion is `as`, exactly like every other
  conversion in [0007](0007-explicit-type-system.md).
- If the grapheme default survives the cost measurement, `string`'s length finally means what the person
  writing `strlen($name)` actually expects it to mean, which is the gap this ADR was opened to close.

**Negative**

- **A further deliberate divergence from PHP**, in the same family as [0007](0007-explicit-type-system.md)
  § 7 but tracked here rather than added to that ADR's table, following the precedent
  [0008](0008-static-and-global.md) set of keeping a decision's own divergence local to it:

  | # | PHP | MWL |
  |---|---|---|
  | 1 | `strlen()` counts bytes; character-aware length needs `mb_strlen()` and a correct `mb_internal_encoding()` | `string`'s default length counts grapheme clusters (or, if the *Revisiting* measurement fails, bytes) unconditionally — there is no second, encoding-sensitive function to get wrong |

- **A real implementation cost if the grapheme default is kept**: an immutable string's grapheme count can
  be cached in its header and computed lazily on first use, but concatenation still needs an O(1)
  boundary-correction check at the seam (a grapheme cluster can span the join, e.g. a base letter in one
  buffer and a combining mark in the next) rather than a free sum of the two cached counts. That cost lands
  on the M3 baseline tier's string-building path and needs to be in its budget, not discovered after.
- **Unicode-version sensitivity.** Extended grapheme cluster boundaries are defined by UAX #29 and gain new
  rules when Unicode adds scripts or emoji sequences. Unlike every other primitive in
  [0007](0007-explicit-type-system.md), what counts as "one character" in a `string` is pinned to whichever
  Unicode version `mwl-runtime` embeds, and can change across a runtime upgrade. Worth stating loudly rather
  than discovering it as a surprising changelog entry.
- **If the fallback is taken**, `string`'s default length is byte length, which reintroduces exactly the
  "does length mean what I think" question this ADR set out to remove — without at least the
  `mbstring`-versus-plain **split**, so still a strict improvement over PHP, but not the preferred outcome.

## Alternatives rejected

- **Codepoint count as the default** (Python 3, Java's effective behaviour). Rejected on the same grounds
  the user raised in review: it is a real improvement over bytes, but it still answers "how many Unicode
  scalar values" rather than "how many characters a person sees" — a flag emoji or an accented letter built
  from combining marks would not count as one. It satisfies neither the intuitive mental model fully nor
  PHP's byte-cost model, and is not the fallback named in *Decision § 2*.
- **`bytes` as `array<uint>`.** Rejected in *Decision § 1*: 8 bytes of tagged-value overhead per byte of
  data, plus a per-write element-type check for a type whose whole point is "no per-element structure."
- **`bytes` as a stdlib class rather than a primitive.** Rejected in *Decision § 1*: drags in object
  identity and refcount machinery a flat buffer does not need, and breaks the buffer-level symmetry with
  `string` that lets both types share one COW implementation.
- **One `string` type with a runtime "is this valid UTF-8" flag, instead of two static types.** This is
  PHP's `mbstring` problem again in a milder form: the same binding could hold text-meaningful or
  not-yet-validated data depending on a runtime flag rather than its static type, so a function still cannot
  tell from the signature alone which one it is holding. It is exactly the ambiguity
  [0007](0007-explicit-type-system.md) already rejected for numeric values under "a variable holds anything,
  always."
- **Computing the grapheme count eagerly at every construction site, rather than lazily and cached.**
  Strictly dominated by lazy-and-cached: it pays the O(n) segmentation cost even for a string whose length
  is never asked for, which the lazy version does not.

## Revisiting

**This ADR cannot move from Proposed to Accepted until a guard test lands in
[`benches/abi-probe`](../../benches/abi-probe/)** measuring the cost of extended-grapheme-cluster
segmentation — both the first-touch cost of counting a fresh buffer and the incremental cost of maintaining
the count across concatenation — against a stated threshold, in the same style
[0002](0002-error-propagation.md)'s and [0006](0006-isolated-script-execution.md)'s guard tests hold their
numbers. A reasonable starting point for that threshold: `bytes as string` already pays an unavoidable O(n)
UTF-8 validation pass at every such conversion, and that cost is already accepted; if grapheme counting adds
no more than roughly that same order of magnitude on top of a string's construction, the default in
*Decision § 2* holds as grapheme-based. If it costs substantially more, the fallback in *Decision § 2*
(byte length as the default, grapheme-awareness as an explicitly named function) takes over instead. The
exact multiplier is for whoever writes the guard test, not asserted here.

Also deferred, each needing its own resolution before or alongside M1:

- **`bytes` literal syntax** — resolved in
  [`docs/spec/00-overview.md` § 5](../spec/00-overview.md#5-bytes-no-dedicated-literal): no dedicated
  literal token; `"…" as bytes` covers the valid-UTF-8 case for free, `Core\Bytes::fromHex()`/`::fromBase64()`
  cover arbitrary binary constants. This is a spelling decision only — it does not touch this ADR's own
  Proposed status, which still turns on the grapheme-cost guard test below.
- **Random access by grapheme index.** Sequential iteration is cheap once the boundary logic exists;
  "the k-th character" without iterating needs either an O(n) scan or a cached offset table, and which one
  v1 needs should be decided from real MWL programs, not guessed now.
- **Normalization (NFC/NFD) is explicitly out of scope.** Grapheme-cluster awareness says nothing about
  whether two visually identical strings compare equal — that is a separate question this ADR does not
  touch, exactly as PHP leaves it untouched today.
