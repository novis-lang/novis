```nvs
$point = {x: 1, y: 2};
$box   = {count: 0};
```

Each anonymous object's precise type is a private, compiler-synthesized class with exactly the fields
written, each field's type inferred from its initializer. That class has **no methods**, no
`implements`, no inheritance and no user-reachable name; it needs no constructor, because the anonymous
object assigns every field it declares; and it is an **ordinary object** in every other respect — reference
semantics, `clone`, `serialize` and an isolate crossing all work on it through the existing uniform
mechanism, because it has real declared properties. Field names are ordinary property names, so the
`camelCase` and no-leading-underscore rules apply unchanged.

**Deliberately absent**, to keep "no dynamic properties, ever" intact: no shorthand `{x, y}` — every
field is written `name: value` — and no computed key `{[$expr]: 1}`. Every field name is a static
identifier, and a repeated one is refused.

One grammar wrinkle: a `{` immediately after `=>` starts a block body
(`rule:types/anonymous-function`), and a statement-initial `{` starts a block, so returning or
discarding an anonymous object directly takes parentheses — `fn() => ({a: 1, b: 2})`. Both are closed call sites, not an
open-ended ambiguity.

Nothing hooks a synthesized class: no `Comparable`, no property observer, no method body to write one
in. A program that wants behaviour on a shared bag of values declares an ordinary class.
