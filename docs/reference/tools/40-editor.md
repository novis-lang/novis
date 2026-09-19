---
id: editor
title: "The editor: nvs lsp, nvs lsp-test and the VS Code extension"
summary: the language server — every request it answers, the diagnostics it publishes, the positions it speaks and the secrets it conceals — `nvs lsp-test`, the suite that freezes an editor answer as text, and the VS Code extension that is the reference client
keywords: nvs lsp, language server, LSP, Language Server Protocol, editor, IDE, VS Code, stdio, initialize, hover, go to definition, completion, autocomplete, semantic tokens, syntax highlighting, document symbol, outline, selection range, folding, document link, code action, quick fix, publishDiagnostics, nvs/redactions, secret, position encoding, utf-8, utf-16, nvs lsp-test, .lspt, --coverage, extension, vsix, TextMate grammar, nvs.path, nvs.lsp.enable, nvs.secrets.redact, nvs.taint.mark, reveal secret, restart language server, activation
---

# nvs lsp

    nvs lsp

The language server. It is started by an editor and not by a person: it reads Language Server
Protocol 3.17 frames on standard input and writes them on standard output, so a terminal that runs
it sees nothing and appears to hang. Nothing else in the process writes to standard output — a
stray line there would be read as a protocol frame and end the session.

It takes no arguments. Everything configurable arrives in `initialize`, because the editor owns
those settings and a flag here would be a second, staler copy of them.

One server answers every editor. There is no per-editor logic in it: a client is a shell that
starts the process, forwards requests and renders answers, so what this section describes is what
every editor gets.

## What it answers

The list is closed. Every capability below is declared at `initialize`, and a client never asks for
anything else, because nothing else is offered.

| Request | What comes back |
|---|---|
| `textDocument/publishDiagnostics` | every diagnostic for the open document, with the phase gate below applied. A notification, sent on open, on every edit, and once more with nothing in it when the document is closed |
| `textDocument/hover` | the `///` run attached to the declaration the name resolves to, handed over as Markdown; a `Core` member's registry row; or, where neither exists, the declared type of what the cursor is on — in that order |
| `textDocument/definition` | the declaring span, anywhere in the `require` graph the analysis loaded. The name is resolved by the type checker and read back here, so an imported name lands where the compiler says it does |
| `textDocument/completion` | the members reachable off a receiver whose class was resolved, the static members and constants reached through a class name, an enum's cases after `Type::`, and — at a bare position — the keywords that may be written there with the variables in scope |
| `textDocument/semanticTokens/full` | what every name in the document *is*, for the colour a regular expression cannot choose |
| `textDocument/documentSymbol` | the outline of the entry file: every declaration it writes, as a tree |
| `textDocument/selectionRange` | the chain of nodes the cursor is inside, innermost first, so expand-selection walks the tree |
| `textDocument/foldingRange` | every construct written over three lines or more, plus the comment blocks |
| `textDocument/documentLink` | each `require`'s path literal, with the file it reached |
| `textDocument/codeAction` | the two quick fixes below |
| `nvs/redactions` | the byte ranges the editor must conceal. Novis's own request; the protocol has no shape for it |

Two code actions ship, and both are a fix the diagnostic that reported the problem was already
carrying: the rename that corrects an identifier's casing, and the rewrite of a legacy cast to
`expr as T`. A fix the compiler would have to compute for itself is offered by nothing. They are
`quickfix` actions and are also grouped under `source.fixAll.nvs`, so an editor can run them on
save.

## Diagnostics, and the phase gate

What is published is what `nvs check` reports, filtered: **a file whose parse failed publishes the
parse error alone**. A broken tree makes name resolution and the type check say things about a
program nobody wrote, and mid-edit every document is a broken tree, so the diagnostics from the
phases that read the tree are held back for that file until it parses.

The filter is by code. `E00xx` (the lexer) and `E01xx` (the parser) are the two phases that build
the tree; an error in either suppresses `E03xx` (name resolution) and `E04xx`, `E07xx` and `E08xx`
(the type check) for the same file. Nothing is rewritten and nothing is re-analysed: the walk runs
in full and the editor is shown a subset of it.

Diagnostics are published for open documents. A document that requires another is re-analysed when
that other file changes, so fixing a class in one buffer clears the error in the buffer that used
it.

## Positions, encodings and document sync

A Novis span is a byte offset, and LSP positions are line and character. The two encodings the
protocol allows are negotiated at `initialize`: **`utf-8` is taken whenever the client offers it**,
and `utf-16` — the protocol's own default — is what a client that offers nothing gets. Every
column is converted for that client and for no other, so a multi-byte line reports the same place
to both.

The document is synchronized in full, not by range. Each analysis reparses the whole document
anyway, so applying a range edit here would buy nothing and add a second place a document's text
could be wrong.

## Semantic tokens

Colour is two layers, and this is the second one. A TextMate grammar in the editor colours what a
regular expression can see — a keyword, a string, a type position — and the server colours what
only a parse can answer, staying silent wherever saying would mean guessing.

The legend it declares, in the order that **is** the wire encoding:

```text
namespace  class  interface  enum  enumMember  type
method  property  parameter  variable  typeParameter
```

with three modifiers: `defaultLibrary`, which marks a `Core` class so the standard library is
visibly not your code, and `tainted` and `secret`, which carry the two value qualifiers to every
use site. The last two are Novis's own, so an editor has to map them to a colour itself; the
others are the protocol's own names and every theme already styles them.

## Concealed secrets

`nvs/redactions` takes a document and answers one list of ranges, each with its kind. The
`secretLiteral` ranges are the ones an editor must not show — the literals and interpolation slots
that flow into a `secret` binding. The `taintedDeclaration` ranges are the opposite instruction: the
name of each declaration whose type carries `tainted`, marked with a glyph and never concealed, and
only where `nvs.taint.mark` asks for it. A credential on a shared screen is an incident; a tainted
value on one is not, and the marker is teaching rather than a default. The server decides both,
never the client: a client that matched `password` against a variable name would be a second, weaker
definition of what a secret is.

A value is concealed because of where it is *written to*, not because of what it looks like. A
literal has no qualifier of its own; the binding it is assigned into carries `secret`, and that is
the question this request asks at every literal in the file. It is deliberately not the semantic
token channel with a modifier on it — that channel degrades to the plain token type when a client
does not understand a modifier, and the failure mode here would be the value becoming visible.

## What it does not answer

Occurrence highlighting, find-references, rename, workspace symbol search and inlay hints are each
one query against a workspace-wide symbol index, which this server does not build. No capability is
declared for them, so an editor never offers the command rather than offering one that fails. Call
hierarchy needs a different index again — call-site edges — and is not built either.

# nvs lsp-test

    nvs lsp-test <paths>... [--coverage]

The editor-behaviour suite. Each case is a document, a cursor, a request and the answer frozen as
text; the paths are case files, or directories walked for `*.lspt`. It prints `N passed, M failed`
and exits non-zero if any case failed.

It is a different suite from `nvs test`, and shares no summary with it: one number that meant both
"the language does what it says" and "the editor answers what it should" would mean neither.

## What a case looks like

```text
--TEST--
a `Core`-owned enum answers its cases off its name alone
--FILE--
<?nvs
var $o = Core\Order::<|>
if (true) {
--REQUEST--
completion
--EXPECT--
Asc     enumMember 0
Desc    enumMember 1
```

- `--TEST--` is one line saying what the case pins.
- `--FILE--` is the document. `--FILE <relative/path>--` writes another file beside it, which is
  how a case reaches across a `require`.
- `<|>` is the cursor: exactly one, removed before the document is analysed, and none at all for a
  request that needs none.
- `--REQUEST--` is one line — the request's name, then any `key=value` arguments.
- `--EXPECT--` is the answer, exact and byte for byte.

**The document usually does not parse, and that is the point.** The unclosed brace in the case
above is what a real buffer looks like halfway through a keystroke, and a suite that only ever
asked about valid code would not be testing the thing the editor is for.

## The requests a case may ask

`diagnostics`, `hover`, `definition`, `completion`, `semanticTokens`, `documentSymbol`,
`selectionRange`, `foldingRange`, `documentLink`, `codeAction`, `redactions`. Four arguments exist:

| Argument | On | What it does |
|---|---|---|
| `phase=all` | `diagnostics` | switches the phase gate off, so the case sees everything the walk reported |
| `prefix=` | `completion` | narrows the list to the labels starting with it, as typing would |
| `limit=` | `completion` | keeps the first N entries of a list already sorted by label |
| `types=` | `semanticTokens` | narrows the answer to those token types |

## How an answer is written down

The rendering is the server's own, so no case invents a spelling and no expectation depends on
where a walk happened to visit first. A diagnostic is `L:C-L:C severity CODE message` sorted by
position; a definition is `file:L:C`, naming the file as the case wrote it; a completion is its
label, kind and detail in three columns, sorted by label; a semantic token is `L:C+length type
modifiers`; a fold is `L-L kind`; an outline is one indented line per symbol; a hover is its
Markdown verbatim. An answer with nothing in it is `none`.

Positions are 1-based here, in both columns — a case is read by people.

```text
--TEST--
a file whose parse failed publishes the parse error and not the cascade under it
--FILE--
<?nvs
var $broken = ;
var $other = $nope;
--REQUEST--
diagnostics
--EXPECT--
2:15-2:15 error E0102 expected an expression
```

## Coverage

    nvs lsp-test --coverage tests/lsp/

prints the request × construct matrix instead of the summary line: which kinds of syntax each
request has actually been asked about. Coverage is read off the node each case's question landed
on and is never declared by the case, so the matrix is a reading of the corpus rather than a list
somebody maintains — and a case added to a directory counts the moment it is written, with nothing
to register.

# The VS Code extension

    editors/vscode

The reference client, and what a person installs to write Novis in VS Code. It colours a `.nvs` file
the moment the file opens, starts `nvs lsp` behind it, and shows what that server answers. It is
built into an installable `.vsix` and is on no marketplace, so installing it means being handed that
file rather than searching for a listing.

It holds no knowledge of the language. Every diagnostic, completion, hover, definition and semantic
colour it shows arrived over the protocol from the server described above; its own source starts a
process, reads settings and draws. A second answer to what a name means — one written here, in
TypeScript, beside the compiler's — would be a thing to keep in step forever and would be the wrong
one first.

## What it claims

`.nvs`, and nothing else. It wakes on a Novis document and stays asleep in a window without one. It
does not claim `.php`, though the compiler reads that dialect: the file type belongs to whichever PHP
extension a person already has, and losing a quiet fight over it looks like Novis being broken rather
than like two extensions disagreeing.

A second language is registered for `.nvst` and `.lspt` — the two case formats above — coloured and
nothing more, so a case reads as the program inside it. No server is started for one.

## Colour arrives twice

A TextMate grammar ships in the extension and colours what a regular expression can see: keywords,
strings, comments, type positions, the `<?nvs ?>` and `<?= ?>` openers and the HTML around them. That
is what makes a file legible before any process has started. Semantic tokens from the server land on
top once it answers, correcting the places only a parse can decide — which of two identically spelled
names is a class, which is a parameter, which value carries `secret`.

Both layers ship *names*, never colours. Every scope is a standard TextMate name suffixed `.nvs`,
every token type is one of LSP's own, and the extension contributes no theme and no colour
customization of any kind. Your theme keeps its opinions about what a keyword looks like. The two
modifiers Novis adds are not in LSP's legend and cannot be, so a theme that has never heard of
`tainted` styles the token underneath it — the value is coloured as the variable it is, rather than
as nothing.

## The settings it contributes

| Setting | Default | What it does |
|---|---|---|
| `nvs.path` | `""` | absolute path to the `nvs` binary. Empty means look it up on `PATH` |
| `nvs.lsp.enable` | `true` | whether to run the server at all. Off leaves the TextMate colour and nothing else |
| `nvs.lsp.trace.server` | `"off"` | log the frames exchanged with the server into the Novis output channel — `off`, `messages`, or `verbose` for the frame bodies too |
| `nvs.check.scope` | `"workspace"` | which files the server reads, so which types completion offers and which files diagnostics are published for — `workspace` for every file under the workspace folder, which is also the only scope an unreferenced private member is dimmed at, `open` for the open documents and what they require or autoload |
| `nvs.codeLens.enable` | `true` | whether a declaration carries its reference, implementor and override counts as a lens |
| `nvs.template.services` | `true` | whether the editor's own HTML, CSS and JavaScript services answer inside an inline-HTML region |
| `nvs.template.format` | `true` | whether the editor's own HTML formatter lays out the markup of a template after `nvs fmt` has laid out the Novis — see below |
| `nvs.secrets.redact` | `true` | conceal the ranges the server reports as `secret` |
| `nvs.taint.mark` | `"off"` | whether a `tainted` value carries a marker glyph as well as the token modifier every theme already styles — `off`, `declaration` for each declaration whose type carries it, or `sink` |
| `nvs.completion.phpNames` | `"all"` | which PHP built-ins are offered beside a half-written name — `all`, `resolved` for only the ones whose `Core` member exists, or `off`. Whatever the value, an item inserts a member only where the registry holds it |
| `nvs.lsp.debounce` | `150` | milliseconds a keystroke is to wait before analysis starts. Contributed and not yet read — see below |

Changing `nvs.path` or `nvs.lsp.enable` restarts the server, since neither can reach one that is
already running.

The rest divide by who reads them. `nvs.secrets.redact`, `nvs.taint.mark`, `nvs.template.services`
and `nvs.template.format` are the client's own and take effect on the next redraw, request or save,
and `nvs.lsp.trace.server` is read by the LSP client library off the id the server is started under. `nvs.check.scope`, `nvs.codeLens.enable` and
`nvs.completion.phpNames` are the server's: the client hands it the whole `nvs` section once, in
`initialize`, so a change to one of those reaches it when it next starts — **Novis: Restart Language
Server**. There is no `didChangeConfiguration` exchange, because two of them decide what the server
*built* rather than how it answers the next request.

## The commands it contributes

Every one is under the **Novis** category in the command palette.

| Command | Title | What it does |
|---|---|---|
| `nvs.restartServer` | Restart Language Server | stops the server and starts it again, which is also what clicking the status item does |
| `nvs.revealSecret` | Reveal Secret | uncovers the one concealed range under the cursor, in this window |
| `nvs.hideSecrets` | Hide Secrets | conceals every range revealed in this window again |
| `nvs.run` | Run File | contributed and not yet answered |
| `nvs.test` | Run Tests | contributed and not yet answered |
| `nvs.showAst` | Show AST | contributed and not yet answered |

## Which binary answered, and which it refuses

The status item on a Novis document names the server: starting, the version that is answering, or
why nothing is. It is the editor's own language-status surface rather than a bar item of Novis's
making, so it sits where every other language's does.

The client refuses a server outside its own `major.minor` series. It learns the version at
`initialize`, because that is where a language server reports one and there is nowhere earlier to
read it from; a binary from another series is stopped there and is never handed a document, a
request or the editor's attention. The extension and `nvs` ship from one commit, so the versions
agree unless a `nvs.path` points somewhere else on purpose.

## Concealed values, on this side of the wire

`nvs/redactions` decides the ranges; this client draws them. A `secretLiteral` range is blurred in
place — a decoration and not an edit, so the buffer is still the file on disk byte for byte and the
cursor, the selection and every edit address the real text.

There are three ways to uncover one range, and they differ in what they leave behind. Hovering it
offers a **Reveal** link, and `nvs.revealSecret` does the same at the cursor; both stay uncovered
until the editor closes or `nvs.hideSecrets` runs. **Putting the cursor inside a range also
uncovers it, for exactly as long as the cursor is there** — moving out covers it again, and nothing
records that it was open. Each way uncovers the one range: a second secret on the same line stays
covered, the reveal is this window's and not the file's, and nothing is written down, so closing the
editor ends it.

### What the concealment does not reach

The blur is cosmetic, and this list is part of the decision rather than a caveat on it — a redaction
trusted past its reach is worse than none.

- **Workspace search results and quick-open previews** render matching lines outside any editor, so
  no decoration applies to them.
- **Diff and version-control views** can be decorated, but there is no type information for the
  "before" side, so the old value of an edited secret is visible in the review of that edit.
- **The minimap** renders from the buffer.
- **Any other extension's** hover, lens or webview reads the document text directly.
- **A copy of a concealed range copies the plaintext.** Nothing may intercept the clipboard.
- **The file itself** is on disk, in the working tree, and in the history the moment it is
  committed. Concealing a hardcoded credential does not make it less hardcoded.

The blur is also a weaker concealment than an opaque fill would be: the smear is a convolution, so a
recording of the screen carries more of the value than a fill would, and a short low-entropy literal
keeps its shape.

A `taintedDeclaration` range is the opposite instruction and is never concealed — it is a name to
mark, and only where `nvs.taint.mark` asks. The client conceals any range kind it does not recognise
rather than showing it, so a server that grows a third kind cannot leak one through an older client.

## Inside a template

The half of a `.nvs` file that is markup is answered by the editor's own HTML service rather than by
Novis. The boundaries are the server's — `nvs/regions` reports every run of inline HTML, and a
`<?= … ?>` hole splits the markup around it into two runs rather than one span covering it — and the
client forwards completion, hover, the colour picker and the linked ranges that rename a tag into
them. Outside a region nothing is forwarded and Novis answers, which includes the first byte of the
`<?` that closes one.

`nvs.template.services` is `false` to turn all of it off, for a project with its own HTML tooling.

**Formatting is `nvs fmt` first, then the editor's HTML formatter over the markup.** A format request
runs `nvs fmt` over the whole buffer, asks `nvs/regions` where the markup in what it answered is, and
hands the editor's own HTML formatter each chunk: the markup between a `?>` that ends its line and the
`<?nvs` that reopens code, `<?= … ?>` holes included. A chunk's lines start at the indentation of its
`?>` line — which `nvs fmt` has put at the depth of the block it sits in — so markup nests from the
code around it, and the line holding the closing `<?nvs` starts there too. Nesting inside a chunk is
the HTML formatter's, in `nvs fmt`'s four-space unit whatever the editor's `tabSize` says, so a chunk
and the code around it agree on what a level is. A hole's bytes are Novis's and are never edited: a
chunk whose layout would move one is left exactly as written, and `nvs fmt --check` passes over every
file this pass formatted. `nvs.template.format` is `false` to leave the markup alone, and then a
format request is `nvs fmt` and nothing else.

Novis has exactly one formatter of its own either way: the embedded services are not registered as
formatters and the server declares no formatting provider. Two things the HTML service does for an
`.html` file do not reach a region: Emmet abbreviation expansion, and HTML validation.

## What it does not do

No language logic, which is the rule the whole client is shaped by: no parser, no formatter and no
table of what a construct means. What it may depend on is an allowlist, and a test enforces the list
rather than a reviewer.

No UI of its own where the editor already has one. Server health is the language-status item, and
the surfaces still to come are the same bargain — coverage through VS Code's own Testing API, a
profile handed to a viewer that reads the open format, a debugger that is DAP's existing interface.
None of them is a panel Novis builds and maintains.

`nvs.lsp.debounce` is contributed and not yet read: the identifier is frozen so a `settings.json`
written against it keeps working when the surface behind it lands, and until then every edit is
analysed as it arrives.
