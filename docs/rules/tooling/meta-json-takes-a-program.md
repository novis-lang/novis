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

A user class also names its lineage: `extends` is its parent class and `implements` the interfaces it
lists, in the order written, and an interface's `extends` is the list of interfaces it extends. Every one
of them is **fully qualified**, resolved through the declaring file's namespace and `use` imports the way
the compiler resolves it, and each key is omitted when the declaration has none. Only what the declaration
itself writes is listed; an interface reached through the parent is on the parent's entry.

The no-argument form must emit byte-identical output before and after the argument exists: the seam is
one input added to one document, never a fork.
