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

The guard is one predictable branch, emitted only at call sites in a package that declared the
capability optional. A program that uses no optional capability runs no guard, and the static check
emits nothing at all.

`Core\Cap::has` reports what the call site already holds. It is **not** a runtime grant: nothing
widens (`rule:security/no-runtime-grant`).

**Partly on disk.** `Core\Cap::has` exists and refuses a written name outside the roster while
checking. The required/optional manifest split, the guard at an optional call site and the compile
error for a required one have no representation in the tree.

**Declined:**
- **Every ungranted capability degrading at run time**, with no required and optional split. It moves
  every capability failure from the build to production and puts a branch on every `Core` call site on
  the request path. The split keeps refusal as the default and makes degradation something an author
  asks for, per capability, in writing.
