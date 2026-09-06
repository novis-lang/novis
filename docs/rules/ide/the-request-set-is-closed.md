M4B answers nine standard requests and no more, plus exactly one of Novis's own. Each is named because
"minimal" without a list is how scope grows: `publishDiagnostics` (the existing `nvs check` pipeline, at
the negotiated encoding, `code` set, phase-gated per `rule:ide/diagnostics-are-phase-gated`); `hover`
(the declared type, a `Core` member's registry signature row, a declaration's doc-comment run as
Markdown); `definition` (the declaring span anywhere in the resolved `require`/`autoload` graph);
`completion` (keywords by position, members off a resolved receiver including `Core` classes, enum cases
after `Type::`, in-scope variables — no workspace symbol search); `semanticTokens/full`; `documentSymbol`;
and three that are projections of data the tree already holds rather than features built on it —
`selectionRange` (the index's ancestor list is the response), `foldingRange` (the same walk plus comment
blocks out of the trivia layer) and `documentLink` (the resolved path literal of a `require` or `autoload`).
The one non-standard request is `nvs/redactions`
(`rule:security/redaction-ranges-come-from-the-server`), non-standard because LSP has no shape for "do not
show this to the room".

The last three standard ones are admitted on one test — the data structure M4B already builds *is* the
answer — and that test is what keeps the list from drifting toward M10's catalog, where `documentHighlight`,
inlay hints and everything else stay. `codeDescription` is not set: it takes a URL per code and there is
no site to point one at.
