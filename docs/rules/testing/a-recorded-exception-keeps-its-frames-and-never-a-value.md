An uncaught exception in a recorded request or run keeps its stack frames, each a function, a file and
a line, and its `previous` chain, each link a class, a message, a site and its own frames, and never
an argument's value.

**The shape.** `frames` is the thrown value's backtrace, innermost first, in the shape the engine
floor's record already renders from it (`crates/nvs-runtime/src/floor.rs`). `previous` is the chain
of the throwable's `previous` property, the nearest cause first. The failure the isolate hands its host
(`crates/nvs-runtime/src/host.rs`, `Failure`) carries both, so the door that writes the `request`
line, and the point that writes a `run` line, has them without touching the exception object, which
`rule:security/isolate-shares-nothing` keeps on the child's side.

**Two bounds.** At most 64 frames per exception and 8 links in the chain are kept. A longer stack
keeps its innermost 64 frames and sets `frames_cut`, and a longer chain keeps its nearest 8 links
and sets `previous_cut`. A deep recursion then costs a bounded line, and the frames a person reads
first are the ones kept.

**When they are copied.** Only when the exception is uncaught and `[debug] store` is on for the
request or run. A caught exception is not recorded, so a program that throws and catches in a loop
pays nothing. With `store` off, the failure carries no frames, and the one branch that skips the
copy is the one that already skips the line.

**Never a value.** A frame holds no argument and no local, because a trace is a `secret` sink
(`rule:security/redaction-reaches-the-tools-own-renderings`). A message is recorded as the program
wrote it, as the request line's message already is.

- **Memory:** with `store` on, per uncaught exception, at most 64 frames and 8 links of names,
  paths and line numbers, held until the line is written.

**Declined:**
- **A frame's argument values.** An argument is often the very credential, token or personal field
  the program was handling, and the index keeps a record for days.
- **Recording caught exceptions.** A program that uses an exception for control flow would write a
  stack per iteration, and the request page would show failures that never reached the person.
- **No bound on the stack.** A runaway recursion would write a line at the 4 MiB cap and lose the
  rest of its record to elision.
