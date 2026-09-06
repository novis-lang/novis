`nvs` gains no `repl` subcommand and no interactive evaluator, and the subcommand roster is not
reopened for one. A REPL needs three things that are each a language question disguised as a tool: a
top-level scope that survives between inputs, where everything is a class member and there is no top
level to bind into (`rule:classes/no-free-functions-or-constants`); redefinition of a class or member
already compiled, where an artifact is keyed by content hash and redefinition is a cache
invalidation, not an edit; and a printed representation of every value, against
`rule:classes/definite-property-initialization`. Answering them would put a second, looser set of
rules beside the one every compiled program obeys — the shape
`rule:statements/nothing-gets-a-second-name` rules against.

What exists instead, and what the documentation points at when the question is asked:

- **`nvs run file.nvs`** for a script, made cheap by the on-disk artifact cache — the second run of an
  unchanged file compiles nothing.
- **`nvs test`** (`rule:testing/test-attribute`) for the "poke at it until it works" loop, which is
  what a REPL is used for most of the time and which leaves something behind afterwards.
- **`Core\Debug::dump`** (`rule:errors/debug-dump`) for looking at a value.

This is a **decision, not a gap**, and `rule:tooling/python-claims` requires it to be said out loud
wherever Novis is compared to Python. It reopens only on evidence that "what is this value" costs a
build, and what would be built then is a debugger-shaped inspector over a paused isolate, not a
general evaluator.
