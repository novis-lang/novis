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
itself writes is listed; an interface reached through the parent is on the parent's entry. The resolution
is `nvs_hir::resolve_ref`, the function the compiler uses, so a consumer reads a class's lineage without
resolving a name itself, and a consumer that wants every ancestor walks the edges. A static method's
`static` is the member's `kind`, never part of its `signature`, which is the member's own spelling as the
registry writes it.

The no-argument form must emit byte-identical output before and after the argument exists: the seam is
one input added to one document, never a fork.

**Memory:** nothing at run time; this is compile-time output. The document grows by one short string per
lineage edge.

**Declined:**
- **The names as written.** `Item` means `Shop\Item` only in the file that imported it, so a renderer
  would have to repeat the compiler's resolution to link it, which is the second resolver
  `rule:statements/one-function-resolves-every-name` forbids.
- **The transitive closure of every ancestor.** It repeats each edge on every descendant, and a
  renderer could no longer tell a direct parent from an inherited one.
- **`static` inside `signature`.** The signature is the member's own spelling, the same as the
  registry's, where `kind` already carries `static`; adding it would make the two halves disagree.
