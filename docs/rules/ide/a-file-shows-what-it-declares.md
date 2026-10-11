A file whose only declaration is one class, interface, enum or `type` alias, beside nothing but
`namespace` and `use`, shows that kind in the editor's file list as a letter beside its name: `C`, `I`,
`E` or `T`. That is the shape every autoloaded file
has (`rule:programs/one-declaration-per-autoloaded-file`), and `nvs_hir::autoload::sole_declaration` is
the one scan both the shape check and this list read, so the shape the editor shows and the shape the
compiler accepts cannot disagree. A file reached by `require` with the same shape is listed as well: it
declares one thing just as plainly, and asking how a file was reached would need an edge the loaded
file does not keep.

The server answers `nvs/fileKinds` with every such file its index holds and the kind of each. The
request takes no parameters and answers `[{uri, kind}]`, `kind` being `class`, `interface`, `enum` or
`type`. The index records the kind as it reads the file, so an answer is one pass over its file map and
no analysis. The client never finds the kind itself by matching `class` or `enum` in the text, because
every judgement about the language stays in the server (`rule:ide/one-server-two-thin-clients`). The
VS Code client draws the answer as a file decoration, the editor's own surface for a badge on a file
(`rule:ide/the-extension-builds-no-ui-the-editor-already-has`). The icon is unchanged: an editor picks
a file's icon from its name and language, never from its content. The decoration carries no colour,
because a decoration's colour is drawn on the file's name too, and a list of recoloured names is
harder to read than the names alone. With no colour the editor draws the letter in the name's own
colour, dimmed, the way a source-control letter looks on a file with no change colour. The letter and
the tooltip are the whole signal: a theme or a user cannot tell the kinds apart by colour. The stable
API takes a badge of one or two characters and draws no shape around it, and no API changes how the
Explorer shows a file's name.

The list is what the index holds. The client asks when a server starts answering, after
`nvs.checkWorkspace`, and when a Novis document is opened or saved, and several asks close together are
one, since the server does not watch `.nvs` files on disk. A file changed outside the editor shows its
old kind until one of those happens, the same staleness every reader of the index has. The badge shares
the decoration slot with source control, so a modified file may show the source-control badge in place
of its kind. No setting is added.

- **Memory:** one more field per indexed file in the server, for the life of the server process.
  Nothing at run time.

**Declined:**

- **The theme's symbol colour for each kind** (`symbolIcon.classForeground` and its siblings). The
  editor draws it on the name as well as the badge, so in a directory of autoloaded files nearly every
  name turned one of three colours, and the colours no longer marked anything out.
- **A circled or filled letter** (`Ⓒ`, `Ⓘ`, `Ⓔ`, `Ⓣ` and the negative circled letters). The circled
  ones looked worse than a plain letter, and the filled ones were too small to tell apart.
- **A badge drawn as an icon.** It is the proposed `codiconDecoration` API, which an extension published
  to the marketplace cannot use.
- **The kind in the file's name**, such as `Status.enum.nvs`. It would let an icon theme give each kind
  its own icon, but it changes how autoload finds a file, which every program's layout depends on, so it
  is a language decision and not an editor one.
