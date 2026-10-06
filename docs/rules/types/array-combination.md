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
