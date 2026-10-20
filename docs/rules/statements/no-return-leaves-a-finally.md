`return` inside `finally` does not compile. It would replace whatever the region was leaving with —
including a throw in flight, which would be discarded with no `catch` anywhere in the program, the
one way an exception could vanish without a handler. The diagnostic names the two rewrites: change
the result in a `catch`, or after the region.

`break` and `continue` whose target lies outside the `finally` are refused on the same grounds; a
loop wholly inside the block keeps both.

`yield` inside `finally` does not compile either (`E0255`). The block also runs with a throw in
flight and while an abandoned generator unwinds (`rule:iteration/generators`), and neither path can
suspend. PHP allows it.

**An error that leaves a `finally` keeps the one in flight.** A `throw` in the block, or a call in it
that throws, is not refused: the error it raises gets the error in flight as its `previous` when its
own `previous` is `null`. When its `previous` is already set, it leaves unchanged and the error in
flight is dropped, because one error has one `previous` and the program set it. A `try` written
inside the block catches first, so an error it handles changes nothing. A `finally` that completes
re-raises the error in flight unchanged.

Whether an override was a bug (usually) or intent is the author's call, so the diagnostic names both
rewrites and picks neither.

**Memory:** none. The checks run in the front end, and the chaining reuses the `previous` slot every
error already has.

**Latency:** one field read and one null test, only on a throw out of a `finally`.

**Declined:**
- **A run-time deprecation warning, with the spelling still allowed.** The line can be refused where it
  is written, and a warning would only permit for a while code that is refused afterwards
  (`rule:expressions/no-keyword-logical-operators`).
- **Refusing a `throw` in a `finally`.** A direct `throw` could be refused where it is written, but a
  call in the block can throw too, and refusing every call there would leave a `finally` that cannot
  close a file. Chaining covers both with one behaviour.
- **Appending the error in flight to the end of the new error's chain.** It would keep both when
  `previous` is set, but it writes into an error the program built and may still hold elsewhere.
