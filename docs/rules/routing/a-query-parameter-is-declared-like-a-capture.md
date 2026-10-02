`#[Query]` on a parameter of a `#[Route]` method declares that the parameter is bound from the request's
query string, by the parameter's own name — the attribute carries nothing that could give it another
key. It is one of the compiler-recognised attributes and is matched nominally
(`rule:core-classes/derive-attribute`): a userland `type Query = {...};` binds nothing, and a `#[Query]`
on a method that declares no route is refused rather than ignored.

The parameter takes the same type list as a path capture and launders the same way
(`rule:security/route-capture-is-laundered-by-its-type`): `string`, `int`, `uint`, `decimal`,
`Core\Uuid`, an enum, a closed set (`rule:routing/a-capture-narrows-to-a-closed-set`), or a class
implementing `Parses`; anything else is a compile error at the parameter. **A default is what makes the key optional**; a parameter without
one is required. Both facts are what the generated API document reads
(`rule:attributes/api-adds-and-cannot-contradict`), and a declared `#[Query]` key is what
`Core\Router::url` accepts beyond the captures (`rule:routing/a-leftover-link-key-is-a-query-string`).

A query value takes no part in choosing a route, so it can never reintroduce the declaration-order
dependence structural precedence exists to prevent — which is what makes the binding safe to add at
all. How a bad value fails is `rule:routing/a-bad-query-value-is-a-400`.
