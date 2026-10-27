`#[Api]` supplies what a handler's types cannot say — tags, error responses, a security scheme, an
example — to the OpenAPI document `nvs build --openapi` writes.

```nvs
#[Route(path: "/orders/{id}", method: Http\Method::Get)]
#[Access(allow: Role::User)]
#[Api(tags: ["Orders"], errors: [{status: 404, type: Api\NotFound}], example: {id: 7})]
public static function show(uint $id): Order { … }
```

**It may add, and it may not contradict.** Each of these is `E0771`, naming both sites: an `errors`
entry whose `type` is not a concrete class of the `Throwable` tree, an `errors` status outside 400 to
599 or listed twice, an `example` on a route whose return type is not a class or a shape, an `example`
naming a field the return type does not declare, an `example` value that does not fit its field's
type (a nested object and each object in a nested array held to their field's class or shape the
same way), a `tags` or `security` entry that is not a string, an `#[Api]` on a method carrying no
`#[Route]`, and a second `#[Api]` on one method. One `#[Api]` describes the operation however many
`#[Route]` attributes the method carries.

**`operationId` is optional and replaces the derived `Class.method`**
(`rule:routing/a-method-is-one-endpoint-everywhere`), for a public API whose clients must keep their
function names across a method rename. It takes the lowercased verb as a suffix exactly as the derived
id does when the method serves several verbs. An `operationId` that another operation in the document
also carries is `E0771` too, naming both sites, because OpenAPI requires the id to be unique.

That list is why the attribute exists. The compiler holds both the annotation and the code it
describes, so the annotation cannot claim a shape the handler no longer returns, which is the normal
state of an unchecked API annotation in a mature codebase. It is also what earns `#[Api]` a place on
the closed list of compiler-recognized attributes: an attribute the compiler could not verify would
not get one.

It is compiler-recognized, matched by name rather than structurally, so a userland anonymous object that
happens to look like one is not one (`rule:attributes/attach-sites-and-forms`).

A `security` name must be one the program declares with `#[Api\Scheme]`, and
`rule:routing/a-security-scheme-is-declared-in-code` holds that comparison and its error.
