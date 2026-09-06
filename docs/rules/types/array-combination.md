Three members combine arrays, and each walks its arguments left to right treating **every key the
same way**:

| Member | Rule | PHP equivalent |
|---|---|---|
| `overlay(array<T> $base, array<U> ...$layers)` | a key already present **replaces** in place; a new key is appended | `array_replace` exactly |
| `underlay(array<T> $base, array<U> ...$layers)` | a key already present is **ignored**; a new key is appended | `$a + $b` exactly |
| `appendAll(array<T> $a, array<U> ...$others)` | every **value** in order, keys discarded; the result is always a list | `array_merge`, for list arguments |

**Key order** is one rule for all three: an existing key keeps its position, a new key lands at the end
in the order first met. That is what makes `overlay` and `underlay` two operations rather than one
with its arguments flipped — `overlay($b, $a)` and `underlay($a, $b)` hold the same entries in
different order, and arrays are insertion-ordered (`rule:types/arrays`), so the difference is
observable.

`overlayDeep` recurses where **both** sides of a key hold an array and **neither is a list**; in every
other case the right-hand value replaces the left wholesale. A list is replaced, never merged
element-wise, because element-wise is the surprise in `array_replace_recursive`.

**`Core\Arr` has no member named `merge`**, in any spelling — the word names two operations in the
language a developer is arriving from — and binary `+`/`+=` with an array operand is a **compile
error** naming `Arr::underlay`. Nothing reproduces `array_merge`; a converter rewrites it by static
type, and diagnoses where the type is not provably a list or a map.

Two related behaviours are stated rather than inherited: `unique`, `diff` and `intersect` compare by
**strict identity**, not by PHP's string cast; and `flip` collapses duplicate values, last occurrence
winning, its result typed `array<string>`.
