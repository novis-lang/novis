M4B answers nine standard requests plus one of Novis's own, and M10 adds eight more standard ones and a
second of its own, each one admitted on the same test: the data structure the milestone already builds *is*
the answer. Each is named because
"minimal" without a list is how scope grows: `publishDiagnostics` (the existing `nvs check` pipeline, at
the negotiated encoding, `code` set, phase-gated per `rule:ide/diagnostics-are-phase-gated`); `hover`
(the declared type, a `Core` member's registry signature row, a declaration's doc-comment run as
Markdown); `definition` (the declaring span anywhere in the resolved `require`/`autoload` graph);
`completion` (keywords by position, members off a resolved receiver including `Core` classes, enum cases
after `Type::`, in-scope variables — no workspace symbol search); `semanticTokens/full`; `documentSymbol`;
and three that are projections of data the tree already holds rather than features built on it —
`selectionRange` (the index's ancestor list is the response), `foldingRange` (the same walk plus comment
blocks out of the trivia layer) and `documentLink` (the resolved path literal of a `require` or `autoload`).
M4B's non-standard request is `nvs/redactions`
(`rule:ide/redaction-ranges-come-from-the-server`), non-standard because LSP has no shape for "do not
show this to the room".

The last three standard ones are admitted on that test rather than on being useful, and the test is what
keeps the list from growing: a candidate either reads a structure the milestone already built or waits for
the milestone that builds it. `codeDescription` is not set: it takes a URL per code and there is no site to
point one at.

M10's eight are admitted the same way, and the structure each one reads is named for the same reason the
nine are. `references` and `documentHighlight` are the workspace symbol index's read side, whole-workspace
and narrowed to the open file; `codeLens` counts that index's uses and walks its extends edge; and
`typeHierarchy` — `prepare`, `supertypes` and `subtypes`, one request in three methods — walks the same
edge both ways. Those four, with unused-member dimming, are
`rule:ide/five-features-are-one-reference-index`'s five readers of the one index. `signatureHelp`,
`typeDefinition` and `implementation` read what `hover` and `definition` already resolve. `inlayHint` is the
eighth, and it is where ADR 0099 § 3's deferral is reversed — for the two idioms
that stopped moving and no others, a `var` declaration's inferred type and a bare literal argument's
parameter name, both read out of the type phase's own tables and never re-derived.

M10's own second non-standard request is `nvs/regions`
(`rule:ide/a-template-region-gets-the-editors-services-and-formatter`), and it passes the same test the
eight do: the lexer already knows where a mode ends, so the boundary list is a projection of a walk the
milestone runs anyway. It is non-standard because LSP has no shape for "these bytes are another
language's" — a client deriving them from a grammar of its own would be a second implementation of the
lexer, which is `nvs/redactions`' argument applied to a boundary rather than to a range. It carries
`.lspt` vocabulary like that one, because `nvs_lsp::render` spells a region the way it already spells a
redaction: a span and one word saying what it is.

Four of the eight carry `.lspt` vocabulary — `codeLens`, `references`, `documentHighlight` and `inlayHint`
are rows in the coverage matrix. The other four are answered on the wire and held by a Rust test, because
`nvs_lsp::render` has no canonical spelling for the shapes they answer and a case may not invent one
(`rule:ide/an-lsp-answer-is-frozen-as-an-lspt-case`). Giving one of them a rendering makes it a case and
changes nothing about the answer.

`textDocument/declaration` is refused rather than deferred: Novis has no declaration that is not the
definition, so it would answer identically to `textDocument/definition`, and the crate names neither
spelling of it.
