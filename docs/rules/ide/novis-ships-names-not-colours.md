Colour is the user's theme's to decide. Both layers ship *names*, and a theme styles only the names it
already recognises, which makes naming the whole of the work and the whole of the risk.

Every scope the TextMate grammar emits comes from the standard TextMate vocabulary, suffixed `.nvs` —
`keyword.control.nvs`, `storage.type.nvs`, `entity.name.type.class.nvs`, `string.quoted.double.nvs`. An
invented name like `keyword.nvs.spawn` is matched by no theme, so the construct renders as unstyled body
text: technically correct and visibly broken. The grammar snapshot test asserts every scope it produces is
on an allowlist of standard names, so a novel one fails in CI rather than in somebody's editor.

Every semantic token type comes from LSP's standard legend. The two modifiers Novis adds are by definition
not in it, so the extension declares `semanticTokenScopes`, mapping each to a standard TextMate scope a
theme already styles — and a theme with no opinion falls back to the underlying token type rather than to
nothing. The extension ships no `configurationDefaults` for `editor.tokenColorCustomizations` or
`editor.semanticTokenColorCustomizations`: whatever a `tainted` value ought to look like is not Novis's
call to make in someone else's editor, which is also what
`rule:ide/tainted-has-no-default-decoration` applies. A bundled theme is a legitimate future option a
user may select; it is not a default.
