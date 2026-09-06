`consume` on an unreachable shared store throws. It does **not** return `allowed: true`.

This is the load-bearing choice. A security control that fails open silently is worse than no
control, because the deployment believes it has one — and whether *this* limiter should fail open or
closed is knowledge only the call site has. A login throttle must fail closed; a plan quota should
fail open rather than take the product down. Throwing puts the decision where the knowledge is, and
an application that chooses to fail open does so in a `catch` that a reader can see and a reviewer
can question.

`shed` cannot fail this way, its state being in the core's own memory, which is one more reason the
two are different members rather than one with a flag.
