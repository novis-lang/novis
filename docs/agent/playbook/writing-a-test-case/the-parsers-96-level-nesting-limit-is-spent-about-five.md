- **The parser's 96-level nesting limit is spent about five levels per bracket, so a case nesting 90
  of them is refused before its first step runs.** `Core\Ast::parse` accepts `echo` behind 15 opening
  brackets and refuses 20, and an uncaught `ParseError` in step 1 means no later step is delivered at
  all. Let the case find the boundary itself — raise the depth in a loop and count the refusals —
  rather than writing a number the grammar can move. [until: reviewed 2026-09-20]
