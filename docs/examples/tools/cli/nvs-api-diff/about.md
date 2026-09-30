`nvs api diff <old.json> <new.json>` compares two OpenAPI documents and lists what changed.

The usual input is the document of the last release and the one that `nvs build --openapi` just
printed. The command prints one line for each change. Each line starts with `breaking`, `additive`
or `cosmetic`. A removed path is breaking, for example, and a new optional parameter is additive.
A summary line with the counts comes last. When the two documents are the same, the command prints
`no change`.

The exit status is `1` when a change is breaking. A build server can use this to stop a release
that breaks the clients of your API.
