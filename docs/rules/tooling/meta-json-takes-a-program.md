`nvs meta --json` with no argument prints the `Core` registry, a static compile-time table, exactly as
`rule:tooling/meta-json` states. With an entry path it prints the same registry plus that program's own
declarations:

```
nvs meta --json                 # the Core registry
nvs meta --json app.nvs         # the Core registry, plus that program's own declarations
```

The second form is **program-dependent** — it parses and resolves the program — which is a materially
different command wearing the same name, so it is stated here rather than discovered. A user declaration
is emitted in a shape mirroring the registry's own: name, signature, the doc comment's prose, its `@see`
list and its `@example` list (`rule:tooling/doc-comment-tags-are-see-and-example`). The `Core` half of that
shape is the registry's and is unchanged by this; the user half is this rule's.

The no-argument form must emit byte-identical output before and after the argument exists: the seam is
one input added to one document, never a fork.
