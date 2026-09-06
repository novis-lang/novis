A closed-world target has no filesystem to fall back on at run time: a bundled executable's payload is
exactly what got embedded and nothing more. `require` therefore resolves **entirely at build time**
inside `nvs build --compile`, and a `require` the build cannot resolve — a target that is not there, or
a path that cannot be known until the program runs — fails the build with a diagnostic naming it.
Never a runtime fallback, never a silent omission, never a failure that first appears on the user's
machine.

`require`'s semantics do not change (`rule:statements/require-is-the-only-inclusion-construct`); only
which paths are legal narrows. At run time the bundle answers every `require` from its embedded table,
and a path the table does not carry is exactly as unloadable as a missing file, reported as the same
diagnostic an ordinary run would give.

This rule is the home for every closed-world target, and any other inherits it rather than restating
it.
