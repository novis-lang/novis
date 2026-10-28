Colour is the user's theme's to decide. Both layers ship *names*, and a theme styles only the names it
already recognises, which makes naming the whole of the work and the whole of the risk.

Every scope the TextMate grammar emits comes from the standard TextMate vocabulary, suffixed `.nvs` —
`keyword.control.nvs`, `storage.type.nvs`, `entity.name.type.class.nvs`, `string.quoted.double.nvs`. An
invented name like `keyword.nvs.spawn` is matched by no theme, so the construct renders as unstyled body
text: technically correct and visibly broken. The grammar snapshot test asserts every scope it produces is
on an allowlist of standard names, so a novel one fails in CI rather than in somebody's editor.

Every semantic token type comes from LSP's standard legend. The two modifiers Novis adds are by definition
not in it, so the extension declares `semanticTokenScopes`, mapping each to a standard TextMate scope a
theme already styles: `*.tainted` to `markup.underline` and `*.secret` to `emphasis`. Both scopes carry a
font style and no colour in the themes VS Code ships, and VS Code resolves each style property from the
rule that sets it, so a qualified variable is underlined or italic and keeps its variable colour. A theme
with no rule for either scope draws the plain token type. The extension ships no `configurationDefaults`
for `editor.tokenColorCustomizations` or
`editor.semanticTokenColorCustomizations`: whatever a `tainted` value ought to look like is not Novis's
call to make in someone else's editor, which is also what
`rule:ide/tainted-has-no-default-decoration` applies.

Colour Novis does choose reaches an editor only when the user picks it, in one of two ways. Both exist
because a popular theme can draw a type and a method in two near-identical shades, and the names alone
cannot fix that. The extension bundles two themes, `Novis Dark` and `Novis Light`, which style the
standard scopes and token types of every language and are never selected for the user. The
`nvs.highlighting.distinctNames` setting, off by default, keeps the user's theme: turned on, it writes
a type's and a method's colour as `:nvs` rules into the user's
`editor.semanticTokenColorCustomizations`, under the theme showing and under each theme switched to
later, and turned off it removes every rule whose value is still the one it wrote. Both use the one
palette in `editors/vscode/src/colours.ts`. The setting's description says both, with a `command:`
link that switches to the bundled theme, because the Settings editor runs such a link in one click.

The extension does turn semantic highlighting on for `.nvs` documents, with the `configurationDefaults`
entry `"[nvs]": { "editor.semanticHighlighting.enabled": true }`. VS Code's own default,
`configuredByTheme`, lets a theme that sets `"semanticHighlighting": false` discard every semantic
token, and many popular themes set it. Under such a theme layer two is gone: an attribute's class, a
named argument and a resolved type show as body text, and the `tainted` underline and the `secret`
emphasis never appear. Turning the setting on ships names and no colour. Without
`semanticTokenColors` of its own, a theme colours each semantic token through the TextMate scope VS
Code maps its type to, so the theme still decides every colour. The setting is read per language, so
no other language's highlighting changes, and a user's own `[nvs]` setting wins over the default.

**Declined:**

- **Leaving the setting to the theme.** The cost is the whole of layer two under every theme that
  opts out, and the user who meets it sees a half-coloured file with no hint of the cause.
- **Writing the rules as a default, or for every theme at once.** A default is a colour the user did
  not pick, and one palette cannot suit both a light and a dark theme, so the rules go under one
  theme's block, in the shade that theme is drawn in.

**Built:** the grammar's allowlist test (`editors/vscode/test/grammar/allowlist.test.ts`), the standard
legend, the `semanticTokenScopes` contribution, the semantic highlighting default, no colour defaults,
the two bundled themes and the `nvs.highlighting.distinctNames` setting in
`editors/vscode/package.json`; `editors/vscode/test/contributions/contributions.test.ts` asserts all
but the first two, and `editors/vscode/test/client/colours.test.ts` asserts what the setting writes and
removes.
