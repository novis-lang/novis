A `string` or `int` value written in type position is a **single-value type** — the type whose only
value is exactly that one — parsed only in type position, the way `array<T>` is. `"a"|"b"|"c"` and
`1|2|3` are ordinary unions of those atoms, canonicalised like any other, and each is **a set of allowed
values**; `?"a"` is sugar for `"a"|null`. They are usable at every binding site
(`rule:types/declaration`), with no special case.

```nvs
function setMode("a"|"b"|"c" $mode) { … }   // the set is the type
```

Assignability and conversion:

| direction | behaviour |
|---|---|
| a single-value type → its base type, and a set of allowed values → its base type | **total, free** — a strict widening, the same representation |
| base type or `mixed` → a single-value type or a set of allowed values | **checked.** Throws unless the value equals one of the named values |
| a wider set of allowed values → a narrower one | needs a guard or a checked `as` — ordinary narrowing (`rule:types/narrowing`) |

**Zero additional runtime representation.** A single-value type shares its base type's tag and payload
exactly; that only the one value is allowed is enforced by the checker wherever the static type is
known. The only place it costs anything is where a value arrives through `mixed` or an isolate boundary,
and the checked conversion runs a membership test against the small, closed, compile-time-known set.

A failed conversion names the accepted set, generated from the type: ``` `"z"` is not one of `"a"`,
`"b"`, `"c"` ``` (`E0469`). That is a **compile** error where the operand settles the question by
itself — the target a closed set, the operand naming one value — and otherwise the ordinary checked
conversion answered at run time. The run-time message shows at most the first 64 characters of a
`string` operand, then `…` and the operand's length in bytes, because the operand is often what a
request sent and a message that repeated all of it would copy that into every log line that records
it. A `tainted` or `secret` value needs the same laundering it would need
to leave `mixed` for any other typed binding; neither qualifier gets a rule of its own here.

There is **no `float` single-value type**, because float equality is imprecise enough that a type
allowing only `0.1` is a footgun.

A parameter that accepts exactly a few values is a value constraint, and untrusted input meets it
through the same reviewable, throwing `as` every other conversion uses. Reusing the union and
conversion machinery, the way `true|false` already sat inside it as `bool`, adds no new kind of thing
to the language.

**Declined:**
- **Wildcard matching over constant or case names** (`Foo::TYPE_*`). The accepted set is not visible
  at the call site without reading `Foo`, and it grows silently the day a new `TYPE_*` constant is
  added. Naming the exact constants (`rule:types/constant-in-type-position`) reuses the group without
  either problem.
- **An attribute listing the expected values.** Attributes are inert metadata
  (`rule:attributes/inert-metadata`), so enforcing one would be a second checking pass beside the
  type checker. It would also lose what a real type has: narrowing, the checked `as`, and static
  knowledge for code generation.
- **A `oneof(…)` or `literal(…)` type constructor.** The union syntax and its canonicalisation
  already exist and single-value atoms compose with them, so a keyword would be a second way to write
  a set of allowed values (`rule:statements/nothing-gets-a-second-name`).
- **A run-time validator call** such as `Core\Arr::contains(["a", "b", "c"], $x)` in place of a type.
  It is not visible in the parameter's declared type, and it is a second, weaker mechanism beside the
  type system.
