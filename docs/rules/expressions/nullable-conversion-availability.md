`as ?T` is available for exactly the conversions the checked form already defines, and is a compile
error everywhere else:

| operand → target | `as ?T` |
|---|---|
| any row the conversion table defines | **available**; `null` where the row throws |
| into a single-value or enum-case type | **available** — the non-throwing twin of a checked conversion |
| from `mixed` | **available**; every target has a checked path from `mixed` |
| a conversion that **cannot fail** (`$i as ?int`, `$i as ?string`) | **compile error** (`E0709`), naming `as T` |
| no conversion exists at all (`array<int> as ?int`) | **compile error** (`E0708`) |
| **any** class or interface target | **compile error** (`E0473`), with no exceptions |

Keep the last three rows straight: **a conversion that exists and failed is `null`; a conversion that
does not exist is a diagnostic.** From `mixed` every conversion exists, so `$mixed as ?int` is `null`
for a value holding an array, while a statically-known `array<int>` never compiles. A `?T` that can
never be `null` is a lie in the type and forces a pointless check downstream, which is why the
cannot-fail row is refused rather than allowed as a harmless spelling. A conversion that does not
exist stays a diagnostic because otherwise `as ?T` would be a universal escape hatch that erases
genuine type errors.

**The class row is absolute and admits no roster of blessed names.** `as ?T` spells a downcast in
every language a reader arrives from, and it never spells a parse: text becomes a value through that
class's own reader (`rule:expressions/try-parse`).

**Declined:**
- **A roster of class targets that parse, such as `$s as ?Core\Uri`.** It inverts what `as?` means in
  Swift and Kotlin, for a few memorized class names while every other class stays a diagnostic. It
  also removes no spelling: `Core\Uri::parse($s)` exists either way, so the operator only moves the
  parse onto the harder of the two to read.
