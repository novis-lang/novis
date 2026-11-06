A file whose only declaration is one class, interface, enum or `type` alias, beside nothing but
`namespace` and `use`, shows that kind in the editor's file list as a letter beside its name: `C`, `I`,
`E` or `T`. That is the shape every autoloaded file
has (`rule:programs/one-declaration-per-autoloaded-file`), and `nvs_hir::autoload::sole_declaration` is
the one scan both the shape check and this list read, so the shape the editor shows and the shape the
compiler accepts cannot disagree. A file reached by `require` with the same shape is listed as well: it
declares one thing just as plainly, and asking how a file was reached would need an edge the loaded
file does not keep.

A file in which a method handles a route shows `R` in place of its kind, and its tooltip names the
class, when the file has the one-class shape, and each route as its verb and path. The first five routes
are named and the others counted. A file with routes is the one a reader looks for when a request
arrives, and it is still a class, so `R` says the more useful of the two. A file of any shape shows `R`,
because a route in a script with other statements serves requests just as one in an autoloaded class
does.

The server answers `nvs/fileKinds` with every such file its index holds. The request takes no
parameters and answers `[{uri, kind?, routes?}]`, `kind` being `class`, `interface`, `enum` or `type`
and absent for a file of another shape, and `routes` every route a method in the file handles, in
source order, as `GET /users/{id}`, and absent when there is none. Both fields are optional so that a
client which knows only `kind` draws what it drew before, and nothing for a file with routes alone. The
index records the kind as it reads the file, and each method's routes off the route table of the
analysis that indexed it, so an answer is one pass over its file map and no analysis. The client never finds the kind itself by matching `class` or `enum` in the text, because
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

The list is what the index holds at the moment of the ask, and the server never settles its queue to
answer it. The client asks the moment a server starts, so settling there would hold every other answer
until the whole workspace was read, which on a first start is minutes. A list asked before the index
is settled is short, and the server sends the notification `nvs/fileKindsChanged`, with no parameters,
each time its queue empties. The client asks again on it, when a server starts answering, after
`nvs.checkWorkspace`, and when a Novis document is opened or saved, and several asks close together are
one, since the server does not watch `.nvs` files on disk. A file changed outside the editor shows its
old kind until one of those happens, the same staleness every reader of the index has. The badge shares
the decoration slot with source control, so a modified file may show the source-control badge in place
of its kind. No setting is added.

- **Memory:** one more field per indexed file in the server, for the life of the server process. An
  answer copies each file's routes, which the index already holds per method, for the length of the
  answer. Nothing at run time.

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
- **A file icon for a file with routes.** An icon theme picks an icon from a file's name, extension and
  language. A second language for route files would split every `nvs` selector, setting and grammar in
  two, and a naming convention for them does not exist.
- **A new `kind` word for a file with routes**, such as `controller`. A client that does not know the
  word draws nothing, so an older extension would lose the `C` on every routed class.
