A `string` literal and an `int` literal are each their own type — the singleton type inhabited by that
exact value — parsed only in type position, the way `array<T>` is. `"a"|"b"|"c"` and `1|2|3` are
ordinary unions of those atoms, canonicalised like any other, and `?"a"` is sugar for `"a"|null`. They
are usable at every binding site (`rule:types/declaration`), with no special case.

```php
function setMode("a"|"b"|"c" $mode) { … }   // the set is the type
```

Assignability and conversion:

| direction | behaviour |
|---|---|
| a literal type → its base type, and a literal union → its base type | **total, free** — a strict widening, the same representation |
| base type or `mixed` → a literal or literal-union type | **checked.** Throws unless the value equals one of the named literals |
| a wider literal union → a narrower one | needs a guard or a checked `as` — ordinary narrowing (`rule:types/narrowing`) |

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
