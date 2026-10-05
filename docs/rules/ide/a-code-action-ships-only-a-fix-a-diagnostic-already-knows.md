A quick fix ships only where its replacement text is already computed, sitting in the
`Diagnostic::suggestions` field `nvs-diagnostics` has carried since M0. The casing fix
(`rule:core-api/identifier-casing`) and the legacy-cast fix `(int)$x` → `$x as int`
(`rule:types/no-legacy-cast`) are admitted for that reason and not because they are useful: the provider is a translation from `Suggestion` to `CodeAction` — a dozen lines and no
new analysis.

The boundary is exactly that. A quick fix whose replacement a diagnostic already knows may ship; one that
would need the checker to compute something new is M10's. Every fix is registered under
`source.fixAll.nvs` as well as `quickfix`, so `editor.codeActionsOnSave` composes them with
format-on-save.

The third fix admitted under the same boundary is the import an undeclared name's diagnostic carries
(`rule:ide/an-undeclared-name-offers-its-import`): the checker computes the `use` line and its place
where it raises `E0303`, so the provider's translation is what it was, and the server still resolves
nothing of its own.

Two code actions are not fixes and have no diagnostic behind them. One is the rewrite of a string or a
`.` chain as an html template (`rule:ide/a-string-converts-to-an-html-template`), filed under its own
`refactor.rewrite.htmlTemplate` kind because applying it changes what the line prints. The other is
*Put on separate lines* and *Join onto one line* (`rule:ide/a-list-splits-onto-lines-and-joins-onto-one`),
which changes only line breaks. Nothing is wrong in either case, so there is no diagnostic to carry
them, and the server computes each from the expression under the cursor alone, with no type or module
question. Neither is ever under `source.fixAll.nvs` or `quickfix`. Any further action the server
computes for itself is still M10's.
