`#[Api]` supplies what a handler's types cannot say — tags, error responses, a security scheme, an
example — to the OpenAPI document `nvs build --openapi` writes.

```nvs
#[Route(path: "/orders/{id}", method: Http\Method::Get, name: "orders.show")]
#[Access(allow: Role::User)]
#[Api(tags: ["Orders"], errors: [{status: 404, type: Api\NotFound}], example: {id: 7})]
public static function show(uint $id): Order { … }
```

**It may add, and it may not contradict.** Each of these is `E0771`, naming both sites: an `errors`
entry whose `type` is not a class a handler could produce, an `example` naming a field the return type
does not declare, a `tags` or `security` entry that is not a string, and an `#[Api]` on a method
carrying no `#[Route]`. One `#[Api]` describes the operation however many `#[Route]` attributes the
method carries.

It is compiler-recognized, matched by name rather than structurally, so a userland anonymous object that
happens to look like one is not one (`rule:attributes/attach-sites-and-forms`).

A `security` name is carried uninterpreted rather than compared: nothing in the compiler or in
`nvs.toml` declares a scheme, so there is no roster to check against and the emitted document names
schemes it does not define. The comparison lands here on the day a scheme has a home.
