`exit` is the only process-termination keyword. `die`, `die()` and `die('bye')` are still recognised — with
the optional `(status)`/`(message)` argument parsed identically — solely so the diagnostic can point at the
fix, and they produce an error node rather than an exit node: *use `exit` instead — it is the only
process-termination keyword Novis keeps.* The code is `E0228`.

The pair had no behavioural difference to preserve: both parsed to one shape with one argument, and both
were type-checked and lowered identically. Of ten other languages with a termination primitive, every one
keeps at most one plain spelling of it; where a second primitive exists it is a genuinely different
behaviour — skips cleanup, cannot be caught, crashes instead of exiting — never a bare synonym. PHP's
`die` is itself a borrowed name without its meaning: Perl's `die` raises a catchable error distinct from
Perl's `exit`, and PHP kept the word but made it a plain duplicate. A second spelling of one construct is
what `rule:statements/nothing-gets-a-second-name` rules out, and rewriting `die(...)` to `exit(...)` is
a rename with the argument unchanged.

**Declined:**
- **`die` as a permanent alias of `exit`.** It differs in nothing, so it buys only a second thing to
  teach.
- **`die` as a second tier, the way Ruby has `exit!` or Python `os._exit`.** No such distinction exists
  in the request or process model to hang a second keyword on, and inventing one only to keep two
  spellings solves nothing.
- **Dropping only one of `die` and `<?php`.** Both are duplicate spellings with no behavioural
  difference (`rule:statements/nvs-is-the-only-open-tag`), so keeping one would need a reason neither has.
