A file whose only declaration is one class, interface, enum or `type` alias, beside nothing but
`namespace` and `use`, shows that kind in the editor's file list: a letter beside its name — `C`, `I`,
`E` or `T` — and the theme's colour for that kind of symbol. That is the shape every autoloaded file
has (`rule:programs/one-declaration-per-autoloaded-file`), and `nvs_hir::autoload::sole_declaration` is
the one scan both the shape check and this list read. A file reached by `require` with the same shape is
listed as well.

The server answers `nvs/fileKinds` with every such file its index holds and the kind of each, and the
VS Code client draws the answer as a file decoration, the editor's own surface for a badge on a file
(`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). The icon is unchanged: an editor picks
a file's icon from its name and language, never from its content. The colour is the editor's
`symbolIcon.*Foreground` colour for a class, an interface, an enum or a type parameter, so the theme
decides it (`rule:ide/novis-ships-names-not-colours`).

The list is what the index holds. The client asks when a server starts answering, after
`nvs.checkWorkspace`, and when a Novis document is opened or saved; a file changed outside the editor
shows its old kind until one of those happens.
