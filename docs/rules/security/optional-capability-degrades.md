A package's manifest splits what it asks for into `required` and `optional`. A **required** capability
that is not granted is a compile error naming the namespace, the capability and the grant line that
would fix it. An **optional** one compiles either way, and each such call site carries a guard that
throws if it is reached — an ordinary catchable throwable, because a package that declared a
capability optional has said it can proceed without it.

The split exists because the compile-time check's granularity is **reachability, not execution**: a
class no name reaches is never part of your program, but inside a class your program does name, every
call site is checked including a branch that never runs. Without the split, the reachability of a
*class* would be the unit of capability granularity, which is far too coarse for a class with two
halves. A library with an in-memory cache and an optional disk-persistence method would fail the build
of every application that never persists anything, and its author's only remedy would be to split
one coherent library into two packages over a capability question.

The guard is decided while compiling, so it costs no branch. The grant table and each package's
`optional` list are in every unit's key, so whether a call site's namespace holds the capability is
known when the unit compiles. A granted call has no guard. An ungranted optional one compiles to a
`RuntimeError` thrown before its arguments run, naming the member, the capability and the grant line
that would allow it. A program that uses no optional capability carries no guard.

`Core\Cap::has` reports what the call site already holds: the capabilities of its namespace's grant
line, handed to the member as its prepared word, that the operator's `nvs.toml` also grants. It is
**not** a runtime grant: nothing widens (`rule:security/no-runtime-grant`). A method reference to
`has` carries no word and answers `false` wherever it is written, which fails closed.

**Partly on disk.** The split, the compile error, the guard and `Core\Cap::has`'s answer are in
`nvs_types::capability` (`reject_ungranted_call`, `prepare_cap_has`), and the guard's throw is
`nvs_ir`'s `lower_capability_guard`. The editor does not read the grant table yet.

**Declined:**
- **A run-time branch at each optional call site**, testing the namespace's grant as the call runs.
  The grant is compile-time input already in the unit's key, so the branch would test a constant.
- **Every ungranted capability degrading at run time**, with no required and optional split. It moves
  every capability failure from the build to production and puts a branch on every `Core` call site on
  the request path. The split keeps refusal as the default and makes degradation something an author
  asks for, per capability, in writing.
