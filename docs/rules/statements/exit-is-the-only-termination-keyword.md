`exit` is the only process-termination keyword. `die`, `die()` and `die('bye')` are still recognised — with
the optional `(status)`/`(message)` argument parsed identically — solely so the diagnostic can point at the
fix, and they produce an error node rather than an exit node: *use `exit` instead — it is the only
process-termination keyword Novis keeps.* The code is `E0228`.

The pair had no behavioural difference to preserve: both parsed to one shape with one argument, and both
were type-checked and lowered identically. Of ten other languages with a termination primitive, every one
keeps at most one plain spelling of it; where a second primitive exists it is a genuinely different
behaviour — skips cleanup, cannot be caught, crashes instead of exiting — never a bare synonym.
