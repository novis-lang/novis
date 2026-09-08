---
id: editor
title: "The editor: nvs lsp and nvs lsp-test"
summary: the language server — every request it answers, the diagnostics it publishes, the positions it speaks and the secrets it conceals — and `nvs lsp-test`, the suite that freezes an editor answer as text
keywords: nvs lsp, language server, LSP, Language Server Protocol, editor, IDE, VS Code, stdio, initialize, hover, go to definition, completion, autocomplete, semantic tokens, syntax highlighting, document symbol, outline, selection range, folding, document link, code action, quick fix, publishDiagnostics, nvs/redactions, secret, position encoding, utf-8, utf-16, nvs lsp-test, .lspt, --coverage
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
