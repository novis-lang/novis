A file whose only declaration is one class, interface, enum or `type` alias, beside nothing but
`namespace` and `use`, shows that kind in the editor's file list as a letter beside its name: `C`, `I`,
`E` or `T`. That is the shape every autoloaded file
has (`rule:programs/one-declaration-per-autoloaded-file`), and `nvs_hir::autoload::sole_declaration` is
the one scan both the shape check and this list read. A file reached by `require` with the same shape is
listed as well.

The server answers `nvs/fileKinds` with every such file its index holds and the kind of each, and the
VS Code client draws the answer as a file decoration, the editor's own surface for a badge on a file
(`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). The icon is unchanged: an editor picks
a file's icon from its name and language, never from its content. The decoration carries no colour,
because a decoration's colour is drawn on the file's name too, and a list of recoloured names is
harder to read than the names alone. With no colour the editor draws the letter in the name's own
colour, dimmed.

The list is what the index holds. The client asks when a server starts answering, after
`nvs.checkWorkspace`, and when a Novis document is opened or saved; a file changed outside the editor
shows its old kind until one of those happens.
