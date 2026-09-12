There is one grammar and one tree. The resilient parse is not a second parser mode or a `rowan`-shaped
CST beside the AST; it is the same `parse` returning a `Parsed { stmts, trivia, modifiers, index }` — the
statements whose spans cover the file, every comment and whitespace run by span in source order, where each
declaration's modifiers were written, and an offset-to-node index. `nvs check`, `nvs run` and every other compile path are that same parse followed by "refuse if
anything was reported", which is what they already do, so no call site changes and nothing has to be kept
in step with a grammar that is still moving.

The parser is already infallible — every production returns a node, never a `Result`, a missing token is
reported at its empty span without consuming what follows, and every bare-sequence loop forces a token of
progress. What the strict tree lacked was the trivia the lexer discards in `skip_trivia`, its single site,
an index, and the span each modifier was written at — recorded by the one loop every declaration's
modifiers already go through, because a `Modifier` says what a declaration is and a formatter putting a
list in one order (`rule:tooling/fmt-base-style-is-per`) has to know where the word is. Those additions
are the whole resilient mode: each is a side channel a compile path collects none of, and none of them is
a node.

The trade is named: without `rowan`'s red/green design there is no free incremental reparse, so each
analysis reparses the document, and `rule:ide/a-full-reanalysis-stays-under-a-bound` is what keeps that a
measured claim. If it fails, the first move is item-level caching over the index, not a second tree.
