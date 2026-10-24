An inspection is an LSP code action, and every one is backed by a diagnostic the checker emits: a casing
violation offers the `camelCase`/`PascalCase` rename (`rule:core-api/identifier-casing`); a legacy
`(int)$x` offers `$x as int` (`rule:types/no-legacy-cast`); a missing constructor property assignment
offers `$this->name = $name;` as the body's first statement where a parameter that is not promoted has
the property's name (`rule:classes/definite-property-initialization`); `include`, `include_once` and
`require_once` offer `require` (`rule:statements/require-is-the-only-inclusion-construct`); a `tainted`
value at a refusing sink offers the laundering call the diagnostic already names, where that call takes
the value as its one argument (`rule:security/tainted-qualifier`); a mis-ordered `tainted secret string`
offers `secret tainted string`; and a `#[Route]` missing its `path` offers the derived one
(`rule:routing/a-quick-fix-writes-a-derived-path`). Each is a suggestion the developer applies
deliberately.

A fix is offered only where the diagnostic knows every word of it. A property with no parameter of its
name has no value to assign, `Core\Ldap\Dn::of` needs an attribute name, and `Core\Secret::reveal` and
`Core\Taint::assertTrusted` need a reason only the author can give, so none of them is offered.

They run against the resilient tree (`rule:ide/the-tree-survives-a-syntax-error`), not a successful
parse: a mis-ordered qualifier, a legacy cast and a `var $x` property are parse or declaration errors,
so a fix that fired only on a clean parse would never fire on the file that needs it.

They are off by default and composable with format-on-save. The extension registers them under
`source.fixAll.nvs`, which VS Code runs through `editor.codeActionsOnSave` independently of
`editor.formatOnSave`, so a developer who opts in gets layout and fixes on one keystroke while `nvs fmt`
itself stays layout-only and `nvs fmt --check` fails for exactly one reason.

A suggestion marked as an **alternative**, one of several edits a person chooses between, is offered
as a quick fix and never under `source.fixAll.nvs`, since a save that applied every alternative would
write one over another. `W1022`'s likely grouping is one (`rule:expressions/misread-grouping-warns`),
and the parentheses that keep the current meaning beside it are not. The mark is a field on the
suggestion, and `nvs check --json` carries it as `alternative` beside `safe`, so the diagnostic says
what it is and no consumer keeps a list of codes. `safe` is a different question: the legacy cast's
fix is not safe and still composes with format-on-save
(`rule:ide/narrowing-is-a-diff-never-a-save-time-fix`), so a fix-all filtered on `safe` would drop it.

A rewrite with no diagnostic behind it is not an inspection. The one there is, a string converted to
an html template (`rule:ide/a-string-converts-to-an-html-template`), is offered beside the quick fixes
under its own `refactor.rewrite.htmlTemplate` kind and never under `source.fixAll.nvs`, since it
changes what the line prints.

**Built:** `crates/nvs-lsp/src/actions.rs` translates every suggestion a diagnostic carries, under
`quickfix` and `source.fixAll.nvs`, and leaves an `alternative` out of the fix-all set. The suggestions
on disk include the casing rename, the legacy cast, the `var` property's visibility and type, an
undeclared name's import (`rule:ide/an-undeclared-name-offers-its-import`), a deprecated use's
replacement, `W1022`'s two groupings and the regrouped bitwise comparison. `E0409` carries the
constructor assignment (`crates/nvs-types/src/ctor_init.rs`), `E0221` the `require` keyword
(`crates/nvs-syntax/src/parser/expr.rs`), `E0116` the reordered qualifiers
(`crates/nvs-syntax/src/parser/ty.rs`), `E0747` the derived path (`crates/nvs-types/src/routes.rs`),
and the `E0401` of a `tainted` URL at an outbound call `Core\Http::allowUrl` around the argument
(`crates/nvs-types/src/expr/args.rs`). That call is marked `alternative`: it checks the address at run
time and may throw, the choice is the reader's, and so `source.fixAll.nvs` never applies it. Each has a
case under `tests/lsp/actions/`.
