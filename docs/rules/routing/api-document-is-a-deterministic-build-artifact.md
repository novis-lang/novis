`nvs build --openapi <file>` runs the program through the same front end `nvs check` uses and
writes the OpenAPI 3.1 document, pretty-printed, to standard output. It is a build artifact and
never a runtime feature: the running server does not construct it, nothing sits on the request
path for a value that changes only when the code does, and serving it is the first-party
framework's job over a file the build produced.

**Two builds of the same source produce byte-identical documents.** The emitter has exactly two
orderings — paths by their written text, operations within a path by lowercased verb — and both are
sorted rather than left to a map, because `rule:routing/api-diff-fails-a-breaking-change` compares
documents and a document that reorders itself between builds makes every diff useless.

OpenAPI 3.1 only, whose schema dialect is JSON Schema, so every declared type maps without loss. No
3.0 downgrade path and no Swagger 2: an older version would mean deciding what to drop when the
type system says something it cannot express. A handler answering HTML or a stream appears as an
operation with an opaque response — honest, and not very useful.
