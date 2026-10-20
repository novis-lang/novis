One mistake in a program reports one error: a name that does not resolve gives its root diagnostic once,
and no later use of it reports again. The checker types every place that has already reported as the
error type. It prints as `mixed` wherever a type is shown, and it is assignable to and from every type.
A member read, a method call, a subscript and an operator on it report nothing and give the error type
again. A union or an intersection with it as a member is the error type.

A value the program *wrote* as `mixed` is not the error type and keeps every error it has
(`rule:types/unions-and-mixed`): a `mixed` parameter used where a `string` is declared is still `E0401`.
Only the place that reported is quiet.

The skip lives in the report helpers, not at their callers. Every `E0401` goes through one function, and
it returns without reporting when either side is the error type, so a report site added later is quiet
for free and a contributor has nothing to remember. The error-count guards that stop a second report
inside one expression stay.

An unresolved `use` reports `E0306` at its own line, and the short name it would have brought in types as
the error type wherever it is written, with no `E0303` there.

The error type never reaches lowering: a program with an error diagnostic does not compile, so `nvs-ir`
meeting it is a compiler bug, and a debug assertion there names this rule.

Why: an editor or a hook that shows the first few errors shows the real ones, and a reader fixes one
line, not every line that repeats it. It costs one type compare per report site, and one interned type
per checker run.

**Declined:**
- **Suppressing by error count alone.** It cannot see past one expression, and it would hide a second,
  independent mistake.
- **Hiding `E0306`.** The `use` line is where the fix is written, so its error is the one to keep.
- **Stopping after the first error.** Two independent mistakes in one program give two errors.
