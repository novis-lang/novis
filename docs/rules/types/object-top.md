`object` is the opaque supertype of every class type — a named declared class, and the anonymous class
an anonymous object synthesizes (`rule:types/anonymous-object`). `object <: mixed`, and every class type
`<: object`.

`object` carries no field or method information statically. It is to the class hierarchy what `mixed`
is to the whole type system, one level narrower: membership in it is structural by construction, but
only because it promises nothing about shape at all, so there is nothing to check. Every shape type is
a subtype of plain `object` (`rule:types/shape-type`), and no class declares anything to satisfy it.

Because `object` names no class, it is not a conversion target — `as object` where a specific class is
meant is refused, and a member reached through an `object`-typed receiver is answered at run time
(`rule:types/erased-member-access`).
