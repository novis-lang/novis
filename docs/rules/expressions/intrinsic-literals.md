A short, closed roster of `Core` methods takes an argument that is really a small program — a regex
pattern, a URI, a date format, a duration, a format string, a SQL placeholder list. When that
argument is a compile-time constant, the compiler **validates it while checking** and **prepares**
whatever the runtime call would otherwise build on first use, storing the derived artifact with the
unit in the artifact cache.

A malformed constant is a diagnostic pointing at the exact offset inside the literal. The check on
`Core\Str::format`'s placeholders also counts and types them against the argument list, which turns a
whole family of `printf`-shaped bugs into compile errors.

"Constant" is the existing definition — a literal, a class constant, or an expression over them. No
new notion is introduced, and **nothing is refused for being dynamic**: an argument that does not
qualify compiles to an ordinary runtime call with ordinary runtime validation. The early error, the
static tier report and the prepared artifact are what the literal form buys, and they are a reason to
prefer it.

This adds no syntax. It is a property of a call whose arguments happen to be literals.
