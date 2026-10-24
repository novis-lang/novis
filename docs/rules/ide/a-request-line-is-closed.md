`--REQUEST--` is one line: the request name, then its `key=value` arguments. The argument set is
closed per request and lives in the case parser, `crates/nvs-lsp/src/case.rs`, so a case cannot ask for something no runner implements.
`completion` takes `prefix=` (filter the labels, which is how a case about `->` avoids freezing the whole
keyword list) and `limit=`; `diagnostics` takes `phase=all` to defeat `rule:ide/diagnostics-are-phase-gated`,
which is how the gating itself gets a case; `semanticTokens` takes `types=` to restrict the rendering to
the token types under test; `rename` needs `to=`, the new name; `willRenameFiles` needs `from=` and
`to=`, the file or folder that moves and where it moves to, each relative to the case's own directory;
`workspaceSymbol` takes `query=`, the text a name has to contain, and asks for every declaration
without it; `codeAction` takes `only=`, a kind of `nvs_lsp::CODE_ACTION_KINDS`, which is how a case
reaches organize imports; the rest take none. A line that leaves out an argument its request needs fails the case.

An unknown request or argument fails the case loudly rather than being ignored. A silently-dropped argument
is a case that passes while testing something else.
