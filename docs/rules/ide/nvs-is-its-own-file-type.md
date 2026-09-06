Every editor client registers `.nvs` as its own file type and language, distinct from whatever PHP
support the editor bundles, and activates on nothing else — the VS Code extension does not activate on
`.php`, and the PhpStorm plugin does not let PhpStorm's PHP plugin or a generic-text fallback claim a
`.nvs` file.

The registration is not polish. Without it PhpStorm's own PHP plugin may take the file, or a plain-text
fallback will, and either failure looks to the user like "the plugin does not work" with no diagnostic
pointing at the real cause. The verification for both clients is the same: opening a `.nvs` file
invokes Novis's client and never the editor's PHP support.

A Novis file is not a PHP file to the editor for the same reason it is not one to the compiler
(`rule:statements/nvs-is-the-only-open-tag`): the grammar, the type syntax and the diagnostics are
different enough that a PHP tool over the file would colour valid Novis as an error and valid PHP that
Novis rejects as fine.
