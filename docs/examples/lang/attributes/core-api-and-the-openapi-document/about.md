`#[Core\Api]` describes a route in the API document that `nvs build --openapi` writes. It goes on a
method that has a `#[Core\Route]`.

Most of the document comes from the code: the path, the parameters, their types and the return type.
The `///` comment above the method gives the summary and the description. `#[Core\Api]` adds the
rest: `tags:` groups the operation, `security:` names the scheme a client must use, `errors:` lists
the error responses, and `example:` shows one answer.

A parameter's type gives its schema. A `Core\Time\Date` is a string with the format `date`, and a
`Core\Time\Instant` has the format `date-time`. A `#[Core\Query] ?T $x = null` is not
required. A `bool` query value is the integer `0` or `1`.

Novis checks `#[Core\Api]` against the code when it compiles. A class in `errors:` must exist, and
every key in `example:` must be a property of the return type. Each name in `security:` must be
declared by a `#[Core\Api\Scheme]` on a class of your program.

**The examples below** add one tag, then errors and a security scheme, then a small API for a client
generator. The fourth shows an `example:` that does not compile. The fifth shows dates and query
values.
