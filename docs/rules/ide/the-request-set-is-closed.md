`nvs lsp` answers twenty-two standard requests, the code actions of
`rule:ide/a-code-action-ships-only-a-fix-a-diagnostic-already-knows` and organize imports, and seven
requests of Novis's own,
each one admitted on the same test: a data structure the compiler or the server already builds *is* the
answer. `crates/nvs-lsp/tests/handshake.rs` holds the declared capabilities to exactly that list, and a
method outside it is answered `MethodNotFound`. Each is named because "minimal" without a list is how
scope grows. The first nine: `publishDiagnostics` (the existing `nvs check` pipeline, at
the negotiated encoding, `code` set, phase-gated per `rule:ide/diagnostics-are-phase-gated`); `hover`
(the declared type, a `Core` member's registry signature row, a declaration's doc-comment run as
Markdown, an `autoload` prefix's namespace and roots, the class the string of `as class<T>` or a
class-name argument names, and the absolute path a path argument names and whether anything is there);
`definition` (the declaring span anywhere in the resolved `require`/`autoload` graph, the string of `as
class<T>` and a class-name argument included); `completion` (keywords by position, members off a resolved
receiver including `Core` classes, enum cases after `Type::`, in-scope variables, inside a `require` or
`autoload` path string or a path argument the entries of the directory its text reaches, inside an
`autoload` prefix the namespaces the workspace declares, inside the string of `as class<T>` the classes
that are a `T`, and inside a class-name argument the classes the program declares — no workspace symbol
search); `semanticTokens/full`; `documentSymbol`; and three that are projections of data the tree already
holds rather than features built on it — `selectionRange` (the index's ancestor list is the response),
`foldingRange` (the same walk plus comment blocks out of the trivia layer) and `documentLink` (the
resolved path string of a `require`, each `autoload` root and `discover` glob, which name the directory
each resolves to and list, an `autoload` prefix, which names the first of its roots that exists, and a
path argument, which names the file or directory the checker resolved it to). A path argument is a string
literal at a parameter the registry marks `CoreTy::Path`, at one a program marks `#[Core\Path]`, or at a
`Core` shape field with the same mark (`rule:programs/relative-paths-resolve-from-their-file`); a
class-name argument is one at a parameter marked `CoreTy::ClassName`. The mark is read off the call the
checker resolved, so a literal at any other parameter is ordinary text.

Each of those answers reads what the compiler already resolved, never a second copy of its rule. An
`autoload` literal's link comes from `nvs_hir::autoload::Site::directories`, which pairs each literal's
span with the directory `AutoloadMap::build`'s own functions resolve it to and keeps only one that
exists; a missing root and a malformed glob get no link. A path argument's link reads the checker's own
join (`ExprTypeTable::path_literal`) for a relative literal and the text itself for an absolute one, so
a relative literal the checker left alone, its first segment holding a `:`, has none; a path that names
nothing on disk has none either, since the program may be about to create it. A directory link is a
`file:` URI ending in `/`, which is what a URI already means by a directory, so the server stays on the
standard protocol; the VS Code client rewrites such a target into its `revealInExplorer` command, and
another client opens it however it opens a directory. An `autoload` prefix hovers as the namespace it
maps, a `{..}` segment shown as the directory name the map uses, and its roots in probe order with a
missing one marked; it has no `definition`, because a namespace has no declaration. The string of `as
class<T>` and a class-name argument hover and jump as the class whose whole name the text is, exactly
as written, because the run-time lookup resolves no `use` line, so `'\App\User'` and a short name
answer nothing.

- **Memory:** one span per `autoload` literal and per prefix, and one `Vec` of parameter marks per
  resolved call, per compile, the `Vec` not allocated for a method that marks nothing. In the server,
  one existence test per literal, root or path argument per link or hover request. Nothing at run time.

The first non-standard request is `nvs/redactions`
(`rule:ide/redaction-ranges-come-from-the-server`), non-standard because LSP has no shape for "do not
show this to the room".

The last three of the nine are admitted on that test rather than on being useful, and the test is what
keeps the list from growing: a candidate either reads a structure that is already built or waits for the
work that builds it. `codeDescription` is not set: it takes a URL per code and there is no site to
point one at, and a link into the repository would point at a Rust constant, which is worse than none.
When there is a site, it is the one field that changes.

The other eight standard requests are admitted the same way, and the structure each one reads is named
for the same reason the nine are. `references` and `documentHighlight` are the workspace symbol index's read side, whole-workspace
and narrowed to the open file; `codeLens` counts that index's uses and walks its extends edge; and
`typeHierarchy` — `prepare`, `supertypes` and `subtypes`, one request in three methods — walks the same
edge both ways. Those four, with unused-member dimming, are
`rule:ide/five-features-are-one-reference-index`'s five readers of the one index. `workspace/symbol`,
the twenty-first, is admitted the same way: it searches the same index's declarations by name, and is
that rule's reader too. A `.lspt` case renders it one line per declaration, `file:L:C kind name`, then
`in` and the class or namespace it is declared in. `signatureHelp`,
`typeDefinition` and `implementation` read what `hover` and `definition` already resolve. `inlayHint` is the
eighth, and it is admitted for two idioms that are settled and no others: a `var` declaration's
inferred type and a bare literal argument's parameter name, both read out of the type phase's own tables
and never re-derived. A site those tables hold nothing for gets no hint: an editor draws a hint as if the
source said it, so a re-derived one is a confident lie. `documentHighlight` reads the index for a reason of its own: it
applies resolution to every occurrence of a name, which is a different walk from resolving the one under
a cursor.

`callHierarchy` is the twenty-second, admitted on the same test, and the user decided to offer it:
`prepare`, `incomingCalls` and `outgoingCalls`, one request in three methods. It reads the call-site
edges that `rule:ide/used-by-counts-the-routes-and-tests-that-reach-a-method` adds to the reference
index, one step at a time, so a recursive pair ends. `nvs_lsp::calls` answers both directions.
`prepare` answers a method and nothing else. An incoming call is each method whose body calls the one
asked about, grouped with its call sites, and each method that sends a `Core\Test::request` its route
matched. An outgoing call is each method the body calls and each handler a request it sends reaches.
A call through a `callable` value or a `mixed` receiver is not shown, because the checker resolved no
single target for it, and neither is a method taken as a callable value.

The second non-standard request is `nvs/regions`
(`rule:ide/a-template-region-gets-the-editors-services-and-formatter`), and it passes the same test the
eight do: the lexer already knows where a mode ends, so the boundary list is a projection of a walk the
analysis runs anyway. It is non-standard because LSP has no shape for "these bytes are another
language's" — a client deriving them from a grammar of its own would be a second implementation of the
lexer, which is `nvs/redactions`' argument applied to a boundary rather than to a range. It carries
`.lspt` vocabulary like that one, because `nvs_lsp::render` spells a region the way it already spells a
redaction: a span and one word saying what it is.

Four of the eight carry `.lspt` vocabulary — `codeLens`, `references`, `documentHighlight` and `inlayHint`
are rows in the coverage matrix. The other four are answered on the wire and held by a Rust test, because
`nvs_lsp::render` has no canonical spelling for the shapes they answer and a case may not invent one
(`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`). Giving one of them a rendering makes it a case and
changes nothing about the answer.

The last three standard requests are the rename of `rule:ide/no-refactoring-introduces-an-alias`, and
they pass the same test. `prepareRename` and `rename` read the reference index for a type and for a
member, with a method's whole override family, and the type phase's record of each body's bindings for
a local. `workspace/willRenameFiles` reads the `require` edges the graph walk recorded, the same edges
`documentLink` underlines, and writes each literal that names a moved file again. A type declared alone
in a file named after it moves that file too, because an `autoload` finds it by that name: the edit is
`documentChanges`, its text edits first and one `RenameFile` last, with the `require` edits that move
makes. A rename the server will not carry out is answered `RequestFailed` with a message the editor
shows: a cursor on no symbol, `$this`, a parameter, a name declared outside the workspace, a new name
that is no identifier or that clashes, and a type whose file must move for a client whose
`workspace.workspaceEdit` capability has no `documentChanges` or no `rename` resource operation. All
three carry `.lspt` vocabulary: `prepareRename` renders a range, `rename` and `willRenameFiles` one line
per edit, a moved file one `moves <from> to <to>` line after them, and a refusal its message.

- **Memory:** nothing held. Each request reads the index and the text of the files its sites are in,
  and a file move analyses each entry point that read the moved file once.

The third and fourth non-standard requests are `nvs/imports` and `nvs/importEdits`
(`rule:ide/a-pasted-type-carries-its-use-line`), and they pass the same test: the first is the
resolution `definition` already applies to a written name, asked over a range instead of a cursor, and the
second is the `use` line completion already writes for an accepted type, written for several names at
once. They are non-standard because LSP has no shape for "what did this text mean where it was copied
from" — a paste is the editor's own operation, and the clipboard is the only channel that fact travels
on. Neither has `.lspt` vocabulary, for the reason the four wire-held requests above have none.

The fifth is `nvs/checkWorkspace` (`rule:ide/check-scope-defaults-to-the-workspace`), and it passes the
test with nothing new built: it is the workspace-scope survey and index build the server already runs at
start, run once more on demand. It is the one request that changes what the server holds rather than
reading it, so it is answered where the store is mutable and never by a `.lspt` case, and its answer is
the number of files the index holds.

The sixth is `nvs/directives` (`rule:ide/the-extension-claims-nvs-only`), and it passes the test with
nothing new built: its table is the default `nvs.toml` the binary already ships, which `bun nv directives
--check-template` already holds to the parser's tree. It is non-standard because the document it completes
is not one the server opens: an `nvs.toml` belongs to the user's TOML extension, so the client sends the
text and the cursor with the request. It has no `.lspt` vocabulary, and `crates/nvs-lsp/tests/directives.rs`
holds it.

The seventh is `nvs/fileKinds` (`rule:ide/a-file-shows-what-it-declares`), and it passes the test with
nothing new built: the index already reads every file it lists, and the kind is the autoload shape check's
own scan, recorded once per file as the index reads it. It is non-standard because LSP has no shape for
"what this file is", and a decoration on a file the editor has not opened is not a document request. It
has no `.lspt` vocabulary, and `crates/nvs-lsp/tests/autoload.rs` holds it.

**Declined:**
- **`textDocument/declaration`.** It is refused rather than deferred: Novis has no declaration that is
  not the definition, so it would answer identically to `textDocument/definition`. That is two
  capabilities and two menu entries for one answer, and the crate names neither spelling of it.
- **Inlay hints for everything an editor could annotate.** Every shape but the two shows an idiom that
  is not settled. A third shape — a `foreach` variable, a
  chained call's intermediate type, a closure's return — is a decision on the same test a new request
  is.
- **Resolving an `autoload` root, or joining a path argument's text, in `crates/nvs-lsp`.** Each is a
  second copy of the compiler's resolution rule, and could disagree with the path the program opens.
- **Marking a directory link in `data` or `tooltip`.** Both are free-form fields a client has to know
  about.
- **`definition` on an `autoload` prefix, to its first root.** A `Location` names a file and a range, and
  a directory is neither; the link already gives the same ctrl-click.
- **Resolving the string of `as class<T>` through the file's `use` lines.** The conversion does not, so
  the editor would jump to a class the program then fails to find.
