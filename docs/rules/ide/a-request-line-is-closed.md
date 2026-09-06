`--REQUEST--` is one line: the request name, then optional `key=value` arguments. The argument set is
closed per request and lives beside the renderer, so a case cannot ask for something no runner implements.
`completion` takes `prefix=` (filter the labels, which is how a case about `->` avoids freezing the whole
keyword list) and `limit=`; `diagnostics` takes `phase=all` to defeat `rule:ide/diagnostics-are-phase-gated`,
which is how the gating itself gets a case; `semanticTokens` takes `types=` to restrict the rendering to
the token types under test; the rest take none.

An unknown request or argument fails the case loudly rather than being ignored. A silently-dropped argument
is a case that passes while testing something else.
