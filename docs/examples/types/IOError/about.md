The error a program throws when something outside it failed: a file is missing, a disk is full, or
a network connection closed too early.

`IOError` is a kind of `RuntimeError`. A `RuntimeError` reports a problem that comes from outside
the program. A `LogicError` reports a bug in the program itself. This matters when you catch the
error. A bug is fixed in the code. An `IOError` is handled where the call is made, because the same
call can succeed later or on another machine. You can use a fallback value, report the problem and
continue, or try again.

Every error has a `message` and the place it was thrown from. An error can also have `previous`,
which is the earlier error that caused it. With both, a report can show what the program was doing
and what went wrong.

**The examples below** show a function that throws an `IOError`, an `IOError` that keeps the first
error in `previous`, and a page that tries three sources in order until one works.
