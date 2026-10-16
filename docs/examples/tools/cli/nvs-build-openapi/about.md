`nvs build --openapi <file>` prints an OpenAPI 3.1 document for the routes of a program.

The document has one `paths` entry for each method with the `#[Route]` attribute. Each entry has
the HTTP method, an `operationId` of the form `Class::method`, the path parameters with their
schemas, and the responses. The `title` of the document is the name of the entry file without its
extension.

The command prints to standard output and runs nothing. Redirect the output to a file to keep it.

**Good to know:** `nvs build` needs `--compile` or `--openapi`. Without one of them it stops with
an error.
