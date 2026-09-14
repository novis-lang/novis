A capture's type may be a **union of `string` or `int` literal types**, or a **subset of an enum's
cases** (`rule:types/literal-types`), beside the scalar and enum types a capture already converts to.
`show("en"|"de"|"fr" $lang)` narrows the segment to a closed set with no grammar of its own: `/fr/docs/intro`
matches, `/xx/docs/intro` does not and falls through to a `404` by the failed-conversion rule
(`rule:security/route-capture-is-laundered-by-its-type`) rather than reaching the handler. The narrowing
is a property of the table, not a check the handler was trusted to write, and the generated API document
emits the set as `enum: [en, de, fr]`.

The constraint is written where the type is written. The inline `{id:uint}` grammar room is deliberately
not taken — the same fact in two places that can disagree — and **a regex constraint is refused
outright, as a security decision**: an application-authored pattern over the request path runs before
any rate limiting, so catastrophic backtracking is a denial of service open to any unauthenticated
client, which is priority 1 spent to buy priority 4 (`rule:programs/memory-priority`). A shape a type
cannot express — a `[a-z0-9-]+` slug — stays a `Core\Validate` check inside the handler, answering `400`.

`Core\Router::url` builds a link for every member of the set and refuses a literal value outside it at
compile time; a computed value is substituted and encoded. Which text an enum case is spelled by — in
a segment, in a link and in the generated document alike — is
`rule:routing/an-enum-capture-is-spelled-by-its-backing-value-or-its-case-name`.
