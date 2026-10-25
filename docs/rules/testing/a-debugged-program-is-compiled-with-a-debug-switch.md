Under `nvs dap`, the baseline compiler compiles the program with one switch on, and nothing else sets
it. With the switch, every local variable lives in a stack slot that a table beside the compiled unit
describes, by its name, its declared type and the slot's place in the frame, and every statement
boundary is a stop site. A stop site tests whether it is armed, by a breakpoint on its line or by a
pending pause or step, with one load and one branch. A stopped frame's variables are read from its
slots, so the debugger shows the value each variable has at that statement.

It is one compiler with a switch, the way the probes are one compiler with a flag word
(`rule:testing/debug-probes`), and not a second tier. The switch is chosen when a debug session starts
and holds for every unit that session compiles, so no request ever runs a unit of the other kind. A
normal run and a production server never set it and pay nothing. A debug session runs slower, because
every local is stored at every statement. A unit compiled with the switch is never written to the
artifact cache, whose key does not cover the switch, and a session compiles its units without reading
the cache.

A site is the statement number the coverage probe already carries, and `nvs_ir::Program::stmt_spans`
maps it to its source span, so a breakpoint's file and line resolve to the sites on that line in each
compiled unit. Under a development `nvs serve`, a recompile after an edit resolves every breakpoint again
against the new unit (`rule:config/an-edit-reaches-the-next-request-without-a-restart`), so a breakpoint
outlives the edit.

Each function compiled with the switch links a frame record to its task on entry and unlinks it on
return: the function, its current stop site and its slot table. A stopped task's call stack is read from
those records and never by unwinding native frames, so the walk is the same on every platform. A
variable's value is shown as the bounded tree a dump renders (`rule:errors/record-transformations`), and
a `secret` value is redacted there as in every other rendering
(`rule:security/redaction-reaches-the-tools-own-renderings`).

- **Memory:** per debugged task, one frame record per call in flight and one stack slot per local. A
  program compiled without the switch holds none of it.

**Declined:**
- **Deoptimising a running frame into a debug version of itself.** It needs a map from every compiled
  frame back to its source state, which is an optimising tier's machinery, and the switch gets the same
  view from the baseline compiler alone.
- **Reading variables out of the native frame through per-value location lists.** A value's register
  or slot changes within one statement and differs per target, which is far more machinery than one slot
  per local in a mode that is already slow.
- **Stopping at the safepoint poll.** The poll sits only at function entry and loop back edges, so a
  breakpoint on most lines would never be reached.
