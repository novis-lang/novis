Calling a method a class does not declare is a compile-time diagnostic, like any other unresolvable
name. There is no `__call` and no `__callStatic`, and neither name can be declared at all — the
method-casing rule refuses a leading underscore before any resolution logic runs.

Nothing replaces them. The requirement is not that the spelling is wrong but that dispatching to a
name the class never declared is: it is invisible from the declaration, unreadable by a checker or an
IDE without reimplementing a runtime dispatch rule, and it turns a mistyped method into behaviour rather
than an error.

A program that wants to handle a family of unknown calls writes an ordinary method taking an explicit
name and argument list, or a `match` keyed by name — visible in the class body and type-checked like
every other call. What that costs is real: a proxy or a fluent facade generated from `__call` has no
mechanical translation and needs a human to write the surface out.
