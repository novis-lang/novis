A shape argument expands at the call site into **one argument per field of the merged field list**: the
arms in declaration order, each arm's fields in declaration order, a name a previous arm already emitted
skipped. Where two arms declare the same name it occupies one slot whose type is the union of their
declarations, which is what lets a discriminant arrive as one value the helper switches on. The options bag
expands through the same function, and a bag is the one-arm, all-optional case of it.

Every slot the written anonymous object does not fill passes a constant — the field's own default where it has one,
and a null otherwise. The helper reads its discriminant first and then reads only the slots that arm
declares, which the type check has already guaranteed are filled. So **no runtime representation of a shape
ever exists** and no helper learns a second calling convention.

What this spends is one ABI slot per field of every arm, paid on every call including the one writing the
smaller arm. That is memory traded for latency and simplicity: nothing is allocated on the path and the
runtime owns no shape value. Arms are a member's own declaration and stay small — a member with enough arms
for this to matter is a member that wanted an enum.

A qualifier classification lands on the **field**, not the parameter
(`rule:core-api/qualifier-behaviour-is-declared`), so a shape's fields are walked rather than exempted.
