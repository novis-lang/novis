`#[Core\Api]` describes a route in the API document that `nvs build --openapi` writes. It goes next to
the `#[Core\Route]` on the same method, and away from a route it is not allowed.

Most of the document comes from the code. The path, the parameters, their types and the return type
are read from the route and the method, and the `/** ... */` comment above the method supplies the
summary and the description. `#[Core\Api]` adds the rest: `tags:` groups the operation, `security:`
names the scheme a client must use, `errors:` lists the error responses, and `example:` shows one
answer.

Novis checks each of those against the code while it compiles your program. A class in `errors:` must
be a class the program declares, and every key in `example:` must be a property of the return type.

**The examples below** add one tag, then errors and a security scheme, then a small API with
everything a client generator needs.
