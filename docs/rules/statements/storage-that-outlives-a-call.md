Exhaustive by construction. A design that needs a slot not on this list changes this rule; it is not an
implementation detail:

| storage | lifetime | declared |
|---|---|---|
| local variable, parameter | the call | `int $n = 0;` |
| class static property | the isolate | `private static int $calls = 0;` |
| class constant | the isolate, immutable | `public const int MAX = 10;` |
| object property | the object | `public readonly uint $id;` |
| top-level script variable | the script's own frame, unreachable from a function | `int $n = 0;` at file scope |

There is no global constant: a constant is never declared outside a class, so "class constant, global
constant" is one row rather than two, and the free-floating half has nothing left to name. There is no
superglobal row either — host-populated request, session and CLI state is the class-static row, filled by
the host at isolate construction instead of by a user initialiser
(`rule:statements/no-host-populated-variables`). An enum case needs no row: it is a compile-time constant
of its enum's integer type, inlined at every use site like any other literal.

The last row is the one to read twice. A top-level `$x` in a `.nvs` file is a local of the script's own
frame and nothing more, so the shared-nothing story holds at file scope for the same reason it holds
everywhere else.
