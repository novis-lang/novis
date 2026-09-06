A package's manifest splits what it asks for into `required` and `optional`. A **required** capability
that is not granted is a compile error naming the namespace, the capability and the grant line that
would fix it. An **optional** one compiles either way, and each such call site carries a guard that
throws if it is reached — an ordinary catchable throwable, because a package that declared a
capability optional has said it can proceed without it.

The split exists because the compile-time check's granularity is **reachability, not execution**: a
class no name reaches is never part of your program, but inside a class your program does name, every
call site is checked including a branch that never runs. Without the split, the reachability of a
*class* would be the unit of capability granularity, which is far too coarse for a class with two
halves.

`Core\Cap::has` reports what the call site already holds. It is **not** a runtime grant: nothing
widens (`rule:security/no-runtime-grant`).

**Partly on disk.** `Core\Cap::has` exists and refuses a written name outside the roster while
checking. The required/optional manifest split, the guard at an optional call site and the compile
error for a required one have no representation in the tree.
