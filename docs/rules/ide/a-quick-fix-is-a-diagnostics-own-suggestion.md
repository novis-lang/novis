An inspection is an LSP code action, and every one is backed by a diagnostic the checker emits: a casing
violation offers the `camelCase`/`PascalCase` rename (`rule:core-api/identifier-casing`); a legacy
`(int)$x` offers `$x as int` (`rule:types/no-legacy-cast`); a missing constructor property assignment
offers to add it (`rule:classes/definite-property-initialization`); `include` and `require_once` offer
`require` (`rule:statements/require-is-the-only-inclusion-construct`); a `tainted` or `secret` value at
a refusing sink offers the laundering call the diagnostic already names
(`rule:security/tainted-qualifier`), a mis-ordered `tainted secret string` included; and a `#[Route]`
missing its `path` offers the derived one (`rule:routing/a-quick-fix-writes-a-derived-path`). Each is a
suggestion the developer applies deliberately.

They run against the resilient tree (`rule:ide/the-tree-survives-a-syntax-error`), not a successful
parse: a mis-ordered qualifier, a legacy cast and a `var $x` property are parse or declaration errors,
so a fix that fired only on a clean parse would never fire on the file that needs it.

They are off by default and composable with format-on-save. The extension registers them under
`source.fixAll.nvs`, which VS Code runs through `editor.codeActionsOnSave` independently of
`editor.formatOnSave`, so a developer who opts in gets layout and fixes on one keystroke while `nvs fmt`
itself stays layout-only and `nvs fmt --check` fails for exactly one reason. PhpStorm's Reformat Code
dialog, with its per-action checkboxes, is the same composition through a different client.

A suggestion marked as an **alternative**, one of several edits a person chooses between, is offered
as a quick fix and never under `source.fixAll.nvs`, since a save that applied every alternative would
write one over another. `W1022`'s likely grouping is one (`rule:expressions/misread-grouping-warns`),
and the parentheses that keep the current meaning beside it are not.

A rewrite with no diagnostic behind it is not an inspection. The one there is, a string converted to
an html template (`rule:ide/a-string-converts-to-an-html-template`), is offered beside the quick fixes
under its own `refactor.rewrite.htmlLiteral` kind and never under `source.fixAll.nvs`, since it
changes what the line prints.
