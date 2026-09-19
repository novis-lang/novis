A function that calls itself without ever returning is stopped. When the call stack grows past the
depth Novis allows, the innermost call throws a `RecursionError`. The limit is on the size of the
stack rather than on a number of calls, so how deep you can go depends on what each call holds. A
small function reaches a few thousand calls.

A `RecursionError` is a kind of `RuntimeError`, so `catch (RecursionError $e)` and
`catch (RuntimeError $e)` both handle it, and the program carries on. The frames are already gone
when the handler runs, so the handler has the whole stack available again and may keep working.
Uncaught, a `RecursionError` ends the program like any other error.

This is what makes a recursive walk over data you did not write safe. Input nested far deeper than
you expected, or a cycle in a table you follow, gives you an error you can report instead of a
crash.

**The examples below** show a runaway recursion caught, a walk a thousand calls deep that finishes,
and a redirect chain that loops back on itself.
