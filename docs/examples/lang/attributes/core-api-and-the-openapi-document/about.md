`#[Core\Api]` describes a route in the API document that `nvs build --openapi` writes. It goes on a
method that has a `#[Core\Route]`.

Most of the document comes from the code. The path, the parameters, their types and the return type
are read from the route and the method, and the `///` comment above the method gives the summary
and the description. `#[Core\Api]` adds the rest: `tags:` groups the operation, `security:` names
the scheme a client must use, `errors:` lists the error responses, and `example:` shows one answer.

Novis checks each of those against the code when it compiles. A class in `errors:` must exist, and
every key in `example:` must be a property of the return type.

Each name in `security:` must be declared by a `#[Core\Api\Scheme]` on a class of your program. Its
`name:` is the name that `security:` uses, and its `type:` is a case of `Core\Api\SchemeType`, such as
`Http` or `ApiKey`. A name that no scheme declares does not compile.

**The examples below** add one tag, then errors and a security scheme, then a small API with
everything a client generator needs. The last one shows an `example:` that does not compile.
