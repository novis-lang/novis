# ADR 0069 — Array combination is key-type-independent

- **Status:** Accepted
- **Date:** 2026-08-24
- **Scope:** how two or more arrays combine into one, and what the result's keys and order are; the removal
  of PHP's `array_merge` and `array + array` rules and the three members that replace them; what
  `{preserveKeys: false}` means; the `Core\Arr` shape corrections that follow from the same audit. Not in
  scope: the array container itself, which is [ADR 0007](0007-explicit-type-system.md) § 5, and the set
  operations `diff`/`intersect`, whose *selection* rule is unchanged here.
- **Amends:** [0007](0007-explicit-type-system.md) § 5 — the illustrative stdlib signature
  `Core\Arr::merge(array<T>, array<U>)` becomes `Core\Arr::overlay`, and the section gains the rule that
  binary `+` and `+=` over two arrays are a diagnostic. [0063](0063-core-api-conventions.md) § 3 — its
  removal item 4 gains `array_merge`, `array_merge_recursive`, `array_walk`, `array_pad`'s negative-size
  mode and the string-cast comparison in `array_unique`/`array_diff`/`array_intersect`.
- **Amended by:** none.
- **Relates to:** 0004, 0011, 0035, 0051, 0053

> **In short:** `array_merge` does two different things depending on a key's *type* — integer keys are
> renumbered and appended, string keys are overwritten — and `$a + $b` does a third thing (left wins) under
> a spelling that looks like arithmetic. MWL has **no integer key at all** ([ADR 0007](0007-explicit-type-system.md)
> § 5), so importing either rule would mean sniffing the *text* of a key to choose a behaviour. Instead there
> are three members, each doing one thing to every key alike: **`Arr::overlay`** (right wins),
> **`Arr::underlay`** (left wins, exactly `$a + $b`) and **`Arr::appendAll`** (keys discarded, values
> appended). `merge` is not a name in the library, `array + array` is a compile error naming `underlay`, and
> no member reproduces `array_merge` — which is the point, since `mwl convert` must choose. The same audit
> makes `{preserveKeys: false}` drop *every* key, splits `Arr::pad` into `padStart`/`padEnd`, renames
> `splice` to `replaceRange` and `combine` to `fromKeysAndValues`, and removes `Arr::each`.

## Context

- [ADR 0063](0063-core-api-conventions.md) fixed the *shape* of every `Core` member but deliberately left
  each member's *semantics* to the spec file. `Arr::merge` was written there with `array_merge` **and** the
  `+` operator in one Replaces cell — two mutually contradictory rules under one name, which is the same
  defect at one remove.
- PHP's rule is genuinely two rules: for each entry of each argument, an integer key is re-appended under
  the next free index and a string key is overwritten in place. A developer must know a key's *runtime type*
  to predict the result, and a mixed array gives both behaviours in one call. It is the most-reported
  surprise in PHP's array surface, and `array_replace` exists precisely because the rule is unusable when
  keys matter.
- **MWL has no integer key.** [ADR 0007](0007-explicit-type-system.md) § 5 makes every key a `string`;
  `$a[8]` is `$a["8"]`. Reproducing `array_merge` here means asking, of each key, "does this string look
  like a canonical decimal integer?" — a content sniff deciding control flow, of exactly the kind § 5
  already refuses for `float`, `bool` and `null` subscripts. The PHP rule is not merely inconvenient in
  MWL; it has no type to hang itself on.
- `$a + $b` is worse than inconvenient: it is arithmetic notation for a set operation, it is silently
  *left*-biased where every other combining spelling in every language is right-biased, and it has no
  compound-assignment sibling that means anything different. It is also common in real code — `$options +
  $defaults` — so it needs a replacement, not just a removal.
- The array surface was audited as a whole while this was decided, since the same key-type-dependent
  thinking appears in `array_slice`'s `preserve_keys` and the same sign-as-mode thinking in `array_pad`.
  Those findings are § 4 rather than a second ADR because each is one line, and all of them are the same
  argument.

## Decision

### 1. Three combining members, each key-type-independent

Each walks its arguments left to right and treats every key the same way.

| Member | Signature | Rule | PHP |
|---|---|---|---|
| `overlay` | `overlay(array<T> $base, array<U> ...$layers): array<T\|U>` | an entry whose key is already present **replaces** the value in place; a new key is appended | `array_replace` exactly |
| `underlay` | `underlay(array<T> $base, array<U> ...$layers): array<T\|U>` | an entry whose key is already present is **ignored**; a new key is appended | `$a + $b` exactly |
| `appendAll` | `appendAll(array<T> $a, array<U> ...$others): array<T\|U>` | every **value** of every argument in order, keys discarded; the result is always a list | `array_merge`, for list arguments |

**Key order** is the same rule for all three: an existing key keeps its position, a new key lands at the
end in the order first met. This is what makes `overlay` and `underlay` two operations rather than one with
its arguments flipped — `overlay($b, $a)` and `underlay($a, $b)` hold the same entries in **different
order**, and MWL arrays are insertion-ordered ([ADR 0007](0007-explicit-type-system.md) § 5), so the
difference is observable in `foreach`, in `Core\Json::encode` and in every `Arr::first`. [ADR 0063](0063-core-api-conventions.md)
R15 is satisfied: two behaviours, two names.

`overlayDeep(array<T> $base, array<U> ...$layers): array<T|U>` recurses where **both** sides of a key hold
an array and **neither is a list**; in every other case the right-hand value replaces the left wholesale. A
list is replaced, never merged element-wise, because element-wise is the surprise in
`array_replace_recursive` — overlaying `[9]` onto `[1, 2, 3]` yielding `[9, 2, 3]` is never what a
configuration merge wanted. `Arr::isList` ([ADR 0007](0007-explicit-type-system.md) § 5's `"0" … "n−1"`
test) is the predicate, so the rule is stated in terms the language already has.

### 2. `merge` is not a name, and `array + array` does not compile

`Core\Arr` has **no member named `merge`**, in any spelling. The word names two operations in the language
a MWL developer is arriving from, so it cannot name one here without the reader having to check which.

Binary `+` and `+=` with an array operand are a **compile error** whose diagnostic names `Arr::underlay`,
alongside the existing arithmetic diagnostic for the other operand types. This is a removal, not a
migration hazard: the operator has no silent behaviour change to fall into, because it stops compiling.

**No member reproduces `array_merge`.** That is the deliberate part. `mwl convert` ([ADR 0011](0011-functions-and-constants-are-class-members.md)
§ 4's style) rewrites it by static type:

| PHP call | Rewrite | When |
|---|---|---|
| `$a + $b` | `Arr::underlay($a, $b)` | always — the semantics are identical |
| `array_merge($a, $b)` | `Arr::appendAll($a, $b)` | every argument is statically a list |
| `array_merge($a, $b)` | `Arr::overlay($a, $b)` | any argument is statically a map |
| `array_merge($a, $b)` | **diagnostic** naming both | the argument type is `mixed` or not provably either |
| `array_merge(...$arrays)` | `Arr::appendAll(...$arrays)` | the one-level flatten idiom |
| `array_merge_recursive($a, $b)` | **diagnostic** naming `overlayDeep` | its scalar-to-array promotion has no equivalent |

The last diagnostic is the honest answer for a converter: `array_merge_recursive` turns two colliding
scalars into a two-element array, which is a data-shape change no typed member can perform.

### 3. `{preserveKeys: false}` discards every key

Wherever the option appears — `slice`, `chunk`, `reverse` — `false` means **the result is a list**, keys
renumbered from `"0"`, and `true` means every key is kept. PHP renumbers integer keys and silently keeps
string ones, which is § 1's defect again in an option name.

`false` stays the default, matching PHP for a list argument, which is what these members are overwhelmingly
called with. For an argument with non-numeric keys the result differs from PHP's: the keys are gone rather
than kept. `mwl convert` records that divergence in its report wherever the argument is not provably a
list, and `{preserveKeys: true}` is the faithful rewrite where it mattered.

### 4. The shape corrections the same audit found

Each follows from a rule already in [ADR 0063](0063-core-api-conventions.md); none is a new principle.

| Was | Is | Why |
|---|---|---|
| `pad(array $a, int $size, T $value)` | `padStart` / `padEnd` | `array_pad` reads a **negative size** as "pad at the front" — sign as mode. R6 already fixed the spelling for `Core\Str` |
| `splice(array $a, int $offset, ?int $length, array $replacement = [])` | `replaceRange` | R8's one range convention, same `(offset, ?length, replacement)` shape as `Str::replaceRange`; `array_splice`'s name describes neither argument |
| `combine(array $keys, array $values)` | `fromKeysAndValues` | R5's `from…` construction verb; "combine" says nothing about which argument is which. Unequal lengths throw (R4) |
| `merge`, `mergeRecursive`, `replace`, `replaceRecursive` | `overlay`, `overlayDeep`, `underlay`, `appendAll` | § 1 |
| `each(array $a, callable $fn): void` | *removed* | `foreach` is the language's own spelling, and R3 forbids the by-reference mutation that was `array_walk`'s only reason to exist. `array_walk_recursive` is a nested `foreach` |
| `countValues(array $a): array<uint>` | `countBy(array<T> $a, {by?: callable}): array<uint>` | `array_count_values` silently skips values that are not `int\|string`; an extractor makes the key explicit and absorbs the userland group-and-count idiom |

Two behaviours are stated in the spec rather than left to PHP's precedent, because in both cases PHP's is a
bug source rather than a decision: **`unique`, `diff` and `intersect` compare by strict identity**, not by
PHP's string cast (`SORT_STRING`), matching what `contains` already promises; and **`flip` collapses
duplicate values, last occurrence winning**, its result typed `array<string>`.

### 5. A key comes back as a `string`

[ADR 0007](0007-explicit-type-system.md) § 5 states it and names `Core\Arr::keys(): array<string>`, so
every key-valued **return** in the spec is `string`: `keys`, `keyOf`, `firstKey`, `lastKey`, `findKey`,
`flip`, and the key half of `toPairs`. A key **parameter** stays `int|string` — that is the subscript
normalisation rule of the same section, where `$a[8]` and `$a["8"]` are one key.

## Consequences

**Positive**

- **A combining call's result is predictable without knowing the data.** Which member was called decides
  it; no key's spelling participates. That is the whole return, and it is unavailable in PHP.
- **`$options + $defaults` survives migration** with its exact semantics *and* its exact key order, under a
  name that says what it does.
- **`Core\Arr` loses two members and gains one** (`merge`/`mergeRecursive`/`replace`/`replaceRecursive`/
  `each` → `overlay`/`overlayDeep`/`underlay`/`appendAll`), and four members get names that predict their
  arguments.
- **Nothing is implemented yet.** `Core\Arr` is nine members on disk, none of them these, so this costs one
  spec edit rather than a migration.

**Negative**

- **`array_merge` has no single target.** The most-called array function in PHP converts to one of two
  members or a diagnostic. This is the deliberate cost of not importing the rule, and it is paid once per
  call site during conversion rather than forever at every read.
- **A mixed-key `array_slice` result differs**, per § 3. Rare, reported by the converter, and the faithful
  rewrite is one option away.
- **Familiarity is spent on `underlay`.** It is not a word PHP developers arrive with. It is exactly R6's
  pair for `overlay`, which is what makes the two learnable together, and the alternative — `defaults` —
  names one use of it rather than the operation.

## Alternatives rejected

- **Keep `Arr::merge` with `array_merge`'s semantics.** Maximum familiarity, trivial conversion. Rejected
  in § 1: it requires a key-content sniff MWL has no type for, and it makes the most common array operation
  the one whose result depends on data the reader cannot see.
- **Keep `Arr::merge` but give it only the right-wins rule.** Tempting — one short familiar name, correct
  behaviour. Rejected because it is the *worst* of the options for the reader it aims at: a PHP developer
  writes `Arr::merge($list1, $list2)` expecting concatenation and gets an element-wise overwrite, silently,
  with no diagnostic possible. A name that changes meaning across languages must not be reused.
- **`Arr::overlay($b, $a)` as the `+` replacement, with no `underlay`.** One member fewer. Rejected in § 1:
  it is not equivalent — the key order differs — so the migration would be silently wrong in an
  insertion-ordered language.
- **`concat` instead of `appendAll`.** What every other language calls it. Rejected by
  [ADR 0063](0063-core-api-conventions.md) R7: members are full words, with a closed abbreviation list this
  is not on, and R7 exists because it removes a per-name judgement call. `appendAll` also pairs with the
  existing `append(array<T> $a, T ...$values)` — values against arrays.
- **Keeping `Arr::each` for symmetry with `map`/`filter`.** Rejected: it can return nothing and mutate
  nothing (R3), so it is strictly a slower `foreach` with a closure allocation, and
  [ADR 0051](0051-standard-library-tiers.md) test 6 rules out a member that restates a language construct.
- **A `preserveKeys` default of `true`.** Lossless, never surprising for maps. Rejected: slicing a list from
  offset 2 would yield keys `"2"`, `"3"`, which is not a list, breaking the overwhelmingly common case and
  every `Core\Json::encode` downstream of it.

## Verification

- **M4S:** conformance cases for each member — `overlay` and `underlay` over the same two arrays asserting
  both the values **and** the key order; `underlay($a, $b)` matching PHP's `$a + $b` byte for byte through
  the differential oracle; `appendAll` over two maps yielding a list; `overlayDeep` recursing into two maps
  and replacing two lists wholesale; `fromKeysAndValues` throwing on unequal lengths; `countBy` with and
  without `by`; `unique`/`diff`/`intersect` distinguishing `"1"` from `1` where PHP's string cast does not;
  `flip` keeping the last of two duplicate values.
- **M4S:** `slice`/`chunk`/`reverse` with `{preserveKeys: false}` over a string-keyed array asserting a list
  result, which is the one place MWL and PHP disagree.
- **M4:** the checker rejects `$a + $b` and `$a += $b` for array operands with a diagnostic naming
  `Arr::underlay`.
- **M4S:** the mechanical spec check of [ADR 0063](0063-core-api-conventions.md)'s *Verification* covers the
  renames for free — R5's verb table (`from…`, `count…`), R6's pairs (`padStart`/`padEnd`) and R7's
  full-word rule all apply to the new names with no new check.
- **M11:** `mwl convert` produces each row of § 2's table, including the two diagnostics, and its report
  lists every `array_slice`/`array_chunk`/`array_reverse` call it could not prove list-typed.
