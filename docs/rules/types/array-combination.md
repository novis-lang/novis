Three members combine arrays, and each walks its arguments left to right treating **every key the
same way**:

| Member | Rule |
|---|---|
| `overlay(array<T> $base, array<U> ...$layers)` | a key already present **replaces** in place; a new key is appended |
| `underlay(array<T> $base, array<U> ...$layers)` | a key already present is **ignored**; a new key is appended |
| `appendAll(array<T> $a, array<U> ...$others)` | every **value** in order, keys discarded; the result is always a list |

**Key order** is one rule for all three: an existing key keeps its position, a new key lands at the end
in the order first met. That is what makes `overlay` and `underlay` two operations rather than one
with its arguments flipped — `overlay($b, $a)` and `underlay($a, $b)` hold the same entries in
different order, and arrays are insertion-ordered (`rule:types/arrays`), so the difference is
observable.

`overlayDeep` recurses where **both** sides of a key hold an array and **neither is a list**; in every
other case the right-hand value replaces the left wholesale. A list is replaced, never merged
element-wise, because two lists merged by position are rarely what a caller meant.

**`Core\Arr` has no member named `merge`**, in any spelling — the word is read as two different
operations — and binary `+`/`+=` with an array operand is a **compile error** naming `Arr::underlay`.

Two related behaviours are stated here too: `unique`, `diff` and `intersect` compare by **strict
identity**, never by a string cast of each value; and `flip` collapses duplicate values, last
occurrence winning, its result typed `array<string>`.

Which member was called decides the result, and no key's spelling takes part, so a combining call is
predictable without knowing the data. PHP's `array_merge` renumbers an integer key and overwrites a
string key, so its result depends on each key's runtime type. With one key type
(`rule:types/arrays`), reproducing it would mean sniffing the text of every key, under a name that
shows the reader neither outcome. A `...$a` spread does keep that reading, but it is written at the
site with both arrays in view, and its two outcomes are the two writes the caller could have written
by hand.

`overlay` is PHP's `array_replace`, `underlay` is PHP's `$a + $b` in both entries and key order, and
`appendAll` is `array_merge` over lists. No member is `array_merge` itself. `+` is arithmetic notation
for a set operation and silently left-biased, yet `$options + $defaults` is common, so it gets a
named replacement and not only a removal. `overlayDeep` tests for a list with `Arr::isList`, a
predicate the language already has. A list overlaid element-wise, `[9]` onto `[1, 2, 3]` giving
`[9, 2, 3]`, is the surprise PHP's `array_replace_recursive` has.

**Declined:**
- **`Arr::merge` with `array_merge`'s rule.** It needs the key-content sniff above, and it makes the
  most common array operation the one whose result depends on data the reader cannot see.
- **`Arr::merge` with only the right-wins rule.** A PHP developer writing `Arr::merge($list1, $list2)`
  expects concatenation and silently gets an overwrite by position. A name that means something else in
  PHP is not reused.
- **`Arr::overlay($b, $a)` as the replacement for `+`, with no `underlay`.** The key order differs, so
  the replacement would be silently wrong in an insertion-ordered array.
- **`concat` for `appendAll`.** A member name is a full word (`rule:core-api/members-are-full-words`),
  and `appendAll` pairs with `append`: arrays against values.
- **`Arr::each`.** It can return nothing and mutate nothing (`rule:core-api/nothing-mutates`), so it
  would be a slower `foreach` with a closure allocation, and a member does not restate a language
  construct (`rule:core-api/tier-placement`).
- **A member for `array_merge_recursive`.** It turns two colliding scalars into a two-element array, a
  change of data shape no typed member can make. `overlayDeep` is the nearest member.
