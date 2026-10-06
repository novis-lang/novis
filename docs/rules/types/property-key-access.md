`$obj->$key` and `$obj->{$expr}` are admitted, and **only** when the operand's type is a `property<T>`
whose argument the receiver satisfies (`rule:types/property-key-variance`). Every other operand keeps
`E0235`, with its help naming `as property<T>`, and the refusal lives in the checker rather than the
parser, because the operand's *type* is the question and the parser cannot see one. The spelling is no
longer what is rejected; the missing check is.

Three neighbours stay exactly as they are: `$obj->$m(...)` is `E0235` forever, because computed
*dispatch* is rejected as a concept and a key is not a method name; `$obj->$key` on a `mixed` or
shape-typed receiver is `E0235`, because there is no `T` to check the key's bound against; and
`unset($obj->$key)` is `E0234`, as `unset` of any property already is.

A **read** through a key is typed as the **union of the set's declared types** — `int|string|?Address`
for a `User` declaring those three — which widens into `mixed` or any covering union without an `as`
and narrows the way every union narrows (`rule:types/unions-and-mixed`).

A **write** is the checked erased store (`rule:types/erased-member-access`): it writes an existing
property, never creates one, and the incoming value is checked at run time against what the class
declares that property to hold. Statically the value must satisfy at least one member of the union — a
value no property of `T` could accept is refused where it is written. Both directions lower to the
erased access the runtime already performs, so per-property hooks and a declared property observer
behave exactly as they do there.

**A write is refused, at the write, where `T`'s public set holds a `readonly` property**, naming it
(`E0782`) — the code an ordinary post-construction write already gets. Reading `$id` through a key is
fine, and a class whose fields are all assignable is unaffected. It is deliberately compile-time:
where a request-controlled name selects a field to write, refusing at build time is the direction
security points in.
