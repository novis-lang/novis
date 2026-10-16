`nvs.check.scope` is `"workspace"` or `"open"`, default `"workspace"`: the symbol index is built over every
`.nvs` file under the workspace folder, or over the open documents and their `require`/`autoload` graph
alone. The index is what completion offers a type out of, what references and the code lens count, and
what diagnostics are published for, so the default is the scope at which a developer is offered the classes
of their own project and not only the ones already on screen. `nvs.checkWorkspace` runs one workspace pass
on demand without changing the setting, which is what a developer who chose `"open"` reaches for. The
command sends `nvs/checkWorkspace`, and the server rebuilds its survey and its index over the whole root.
What that pass added stays in the index until the server restarts, and the dimming below stays silent
under `"open"`, because one pass does not keep the index whole.

Workspace scope is the expensive setting. The tree under the root is walked once when the server starts
and a repository nobody has opened a file in is still read in full; what it holds for the life of the
session is one declaration and occurrence list per file. `"open"` is the answer for a tree too large for
that, and a client that named no workspace folder gets the open documents under either value, because
guessing a root from an open file's parent would index whatever happened to be beside it.

The one feature that is only correct at workspace scope —
`rule:ide/five-features-are-one-reference-index`'s unused-member dimming — is silent at open scope rather
than wrong.

Both identifiers, with `nvs.codeLens.enable`, `nvs.template.services` and the `nvs/regions` request, are
added to the extension's frozen roster under that roster's own rule: a name is added and never renamed.
