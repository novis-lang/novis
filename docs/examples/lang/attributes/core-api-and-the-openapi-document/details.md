`nvs build --openapi main.nvs` writes the document to standard output and runs nothing. The document's
`info.title` is the file name without `.nvs`. `info.version` is the `version` in the nearest
`package.toml`, found from the folder you run the command in. Without one, it is `0.0.0`. A program with no
`#[Core\Route]` writes no document. The command prints a message to standard error that says so, and
exits with status 0. A program that does not compile writes no document either.
The command prints the compile errors and exits with status 1.
Two routes can have the same OpenAPI path: `/files/{path}` and `/files/{path...}` with one HTTP
method, or `/a/{id}` and `/a/{key}` with any two. Then the command prints an error that names both
routes, and writes no document.

The paths are sorted by their text, and the operations of one path by their HTTP method. Two builds of
the same file give the same bytes. Each operation's `operationId` is the full name of its method,
with dots: `Shop.show`, or `App.Shop.show` in the namespace `App`. When one method has several
routes, each `operationId` gets the HTTP method in lowercase after a dot, such as `Hook.receive.post`
and `Hook.receive.put`. `#[Core\Api(operationId: 'listOrders')]` sets the `operationId` yourself.

The first sentence of the `///` comment is the `summary`. The rest of the comment is the
`description`, and a blank `///` line in it stays a blank line. A comment with no full stop is all
`summary`. A method with no `///` comment has neither.

Each capture is a parameter with `"in": "path"`, and it is always required. Each `#[Core\Query]`
parameter is a parameter with `"in": "query"`. It is required when it has no default value. Each type
has its own schema. An `int` is `integer`, and a `uint` is `integer` with a `minimum` of `0`. A
`decimal` is a `string` with the format `decimal`. An enum lists the names of its cases, and a set of
allowed values such as `'x'|'y'` lists those values.

The `200` response has the schema of the return type when it is `string`, `int`, `uint`, `float`,
`bool` or `decimal`. Every other return type has the empty schema `{}`. This includes a class, an
array, an enum and a `?int`. A `void` method has a `200` response with no content.

`#[Core\Api]` has five fields: `tags`, `security`, `errors`, `example` and `operationId`. Every
field is optional, and `#[Core\Api]` with no list is allowed. Any other field is `E0454`. Each value must be written
directly in the code, so `"a" . "b"` in `tags` is `E0725`. You can write `#[Api(...)]` after `use
Core\Api;`. A `static` route method takes `#[Core\Api]` too.

Each check below gives `E0771` when it fails. `#[Core\Api]` on a method with no `#[Core\Route]` is
not allowed. `tags` and `security` must be arrays, even for one entry, and each entry must be a
string. An `errors` entry needs both `status` and `type`, and `status` must be a number from 100 to
599. `example` must be an anonymous object. When the return type is a class, each key of `example`
must be a property of that class.

A class in `errors` that does not exist is `E0303`. A built-in class such as `RuntimeError` works
too. Each `errors` entry becomes a response under its status, and the response's `description` is the
class name. An empty `tags` array writes no `tags`. Each name in `security` becomes its own entry in
the operation's `security` list, with an empty list of scopes. Each name in `security` must be the
`name` of an `#[Core\Api\Scheme]` on a class of the program. Any other name is `E0886`. Each scheme
`type` needs its own fields: `Http` needs `scheme`, and `ApiKey` needs `in` and `parameter`. The
`example` value is written into the `200` response beside its schema.
