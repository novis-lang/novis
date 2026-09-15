An enum capture matches on its case's **written** backing value, and on the **case name** when the
value was counted rather than written, in a path segment and in a `#[Query]` value alike.

Every Novis enum is backed by one integer type and nothing else (`rule:enums/one-backing-type`), so
there is no backed-versus-pure split to inherit; what decides the spelling is whether the integer was
written in the enum body:

- Every admitted case's value written — the segment is that integer, parsed as the enum's backing
  type.
- Any admitted case's value counted on from the one before it (`rule:enums/declaration`) — the whole
  subset is spelled by case name, compared byte for byte and case-sensitively, as a literal segment
  is (`rule:classes/names-resolve-case-sensitively`).

The choice is per subset and never per case, because a subset mixing the two would put an integer and
a name in the same position, and a case may legally be named `7`.

A counted value is not an identifier: `enum Status { Active, Banned }` gives `Active` the value `0`
only because it is written first, so inserting a case ahead of it would repoint `/0` at a different
case and silently change what every link already in the world means. A written value is one its
author stated and intends to keep, which is why it is the one that reaches the URL.

Two cases may carry the same value (`rule:enums/declaration`), and a subset admitting two of them
under the written-value spelling is a **compile error** naming both cases and the value, rather than
a silent first-wins (`rule:errors/ambiguous-input-refused`). Under the case-name spelling an alias is
fine, because the names are still distinct.

A segment naming no admitted case is not a match and falls through to a `404`
(`rule:security/route-capture-is-laundered-by-its-type`), exactly as a failed `int` conversion does.
One spelling serves every position: it is what the match converts, what `Core\Router::url` writes
into a link and refuses outside the set (`rule:routing/link-name-and-params-are-checked`), and what
the generated document lists in `enum: [...]`, so a route, its links and its document cannot
disagree.

**The match half is built and the link half is not.** The spelling is decided while compiling
(`nvs_types::routes`' `enum_capture`) and a segment reaches the handler as its case, in a path
segment; `Core\Router::url` still writes an enum argument as the bare backing integer and checks it
against no set, so a link into a name-spelled subset is not yet the segment the match would claim,
and a `#[Query]` value still arrives as its text.
