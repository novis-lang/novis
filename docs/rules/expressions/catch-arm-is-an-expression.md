The arm body is parsed as an expression, and that is the entire rule for what it may hold.

**`throw` is allowed**, because `throw expr` is already an expression. `f() catch (IOError $e) =>
throw new IOError("count failed", {previous: $e})` wraps and rethrows in one line, and the variable on
the left is never assigned — the same answer `$x = $y ?? throw new …` already gives. A `throw` arm is
typed `never` and so contributes nothing to `rule:expressions/catch-result-type`'s union.

**`return`, `break` and `continue` are refused**, because they are statements. The parser names them
rather than reporting a generic expected-expression: `E0126` — *an arm is an expression; `throw` is
one, `return` is not; for an early return write the block form.*

The refusal is deliberate and not a gap to fill later. A `return null` on an `IOError` hides a
failure, and the block form's five lines are the right price for choosing that.
