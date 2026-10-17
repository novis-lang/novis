Every editor client registers `.nvs` as its own file type and language, distinct from whatever PHP
support the editor bundles, and activates on nothing else — the VS Code extension does not activate on
`.php`, and no PHP extension or generic-text fallback claims a `.nvs` file.

The registration is not polish. Without it the editor's own PHP support may take the file, or a
plain-text fallback will, and either failure looks to the user like "the extension does not work" with
no diagnostic pointing at the real cause. The verification is that opening a `.nvs` file invokes
Novis's client and never the editor's PHP support.

A Novis file is not a PHP file to the editor for the same reason it is not one to the compiler
(`rule:statements/nvs-is-the-only-open-tag`): the grammar, the type syntax and the diagnostics are
different enough that a PHP tool over the file would colour valid Novis as an error and valid PHP that
Novis rejects as fine.

**Built:** `editors/vscode/package.json` registers `nvs` for `.nvs` and claims no PHP file type, and
`editors/vscode/test/contributions/contributions.test.ts` asserts both.
