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

**A class static belongs to the request, never to the process.** Each declared static has one slot per
in-flight request, set from the declaration's own initializer when the request's context is built and
released when it ends. A process-global static is a channel from one request into the next: a token
cached in one is readable by whoever sends the next request, and no care in user code closes that, so
security and request isolation settles it. `nvs run` cannot tell the two apart, because a CLI run is
one request, and `nvs serve` is where the difference shows. What it costs is PHP's process-lifetime
memo, which `rule:security/isolate-shares-nothing` already removes. A cache that must outlive a
request is a `Core` capability with a stated lifetime, not a class variable. A static has no
constructor to assign it, so its declaration carries an initializer unless its type admits `null`
(`E0409`), and a nullable one starts each request at `null`.

- **Memory:** one slot per declared static per in-flight request, released with the request, so the
  total is O(in-flight) and never grows with requests served.

The last row is the one to read twice. A top-level `$x` in a `.nvs` file is a local of the script's own
frame and nothing more. The script body is a function, so its variables are locals, and with no
`global` no other function can see them (`rule:statements/no-function-static-and-no-global`). The
shared-nothing story holds at file scope for the same reason it holds everywhere else.
