`#[Query]` on a parameter of a `#[Route]` method declares that the parameter is bound from the request's
query string, by the parameter's own name — the attribute carries nothing that could give it another
key. It is one of the compiler-recognised attributes and is matched nominally
(`rule:core-classes/derive-attribute`): a userland `type Query = {...};` binds nothing, and a `#[Query]`
on a method that declares no route is refused rather than ignored.

The parameter takes the type list of a path capture and launders the same way
(`rule:security/route-capture-is-laundered-by-its-type`): `string`, `int`, `uint`, `decimal`,
`Core\Uuid`, `bool` (written `1` or `0`, which a path capture may not declare), an enum, a closed set (`rule:routing/a-capture-narrows-to-a-closed-set`), or a class
implementing `Parses`; anything else is a compile error at the parameter. **A parameter is filled by a
capture or by the query string, never both:** a `#[Query]` parameter that a capture of the route's path
also names is `E1009` at the parameter, and its help says to delete `#[Query]` or the capture. So every
key of `Core\Router\Match::parameters()` names one capture or one query value, and no order between
the two is needed. **A default is what makes the key optional**; a parameter without
one is required. **`?T $x = null` is the default `null`:** the key may be absent, and then the value is
`null`; a present key converts as `T`, and a value `T` refuses is the same `400`. It works for every type
on the list, a `Parses` class included, and it is row 4 of `rule:core-api/required-optional-and-nullable`.
A query string cannot carry `null`, so `?T` with no default, or with a default that is not `null`, is
`E0752`, and its help says to write `= null`. The route row records the parameter's type as `T`, so the
API document lists the key as optional and typed by `T`. Both facts are what the generated API document reads
(`rule:attributes/api-adds-and-cannot-contradict`), and a declared `#[Query]` key is what
`Core\Router::url` accepts beyond the captures (`rule:routing/a-leftover-link-key-is-a-query-string`).

A query value takes no part in choosing a route, so it can never reintroduce the declaration-order
dependence structural precedence exists to prevent — which is what makes the binding safe to add at
all. How a bad value fails is `rule:routing/a-bad-query-value-is-a-400`.

**Declined:**
- **No query binding, and no query parameters in the generated document.** Every endpoint would
  validate its own query by hand, and the document would miss most of its parameters, which is most
  of what a client author reads it for.
- **`?T $x = 1`, a nullable query value with a default that is not `null`.** A query string cannot
  carry `null`, so the `?` would say nothing.
- **A `query:` field on `#[Route]`.** It puts a list of bindings away from the parameters it names. A
  sibling attribute is how `#[Route]` is extended, as `#[Access]` already is.
