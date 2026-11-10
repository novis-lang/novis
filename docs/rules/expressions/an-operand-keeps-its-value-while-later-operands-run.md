Operands are evaluated left to right, and a value an earlier operand read stays alive until its
consumer has used it, whatever a later operand does. `$this->s . $this->replace()` concatenates the
string `$this->s` had when it was read, even when `replace()` assigns a new string to `$this->s` and
the old one has no other owner.

A read of storage the frame does not own is **borrowed**: it costs no reference count. The lowering
gives it a reference of its own only when an operand lowered after it, and before its consumer uses
it, can free it. One function in the lowering lowers every operand, given the expressions that follow
it, and the consumer names its site from one closed list, so no consumer judges a read alone.

**Whether a later operand can free a read is syntactic, in two halves.** Both halves are one
exhaustive match over the expression kinds with no catch-all arm, so a new expression kind does not
compile until it is judged.

- **A read through a property or a static property** is shared storage: any call, hook or
  destructor can replace it. It is held when any later operand is something other than a literal or a
  bare local read.
- **A read of a bare local, or of an element chain rooted at a local with no property in it,** is
  storage only this frame writes. It is held only when a later operand writes that local: an
  assignment, a compound assignment, `++` or `--`, a destructuring target, or an `inout` argument.

So `$this->a . $this->b` and `$s . f($x)` cost nothing more. Where a form might write a local in a
way the scan cannot see, the local is judged as shared storage, at one more pair per operand.

**What it spends:** one non-atomic increment and one decrement per held operand, per evaluation, on
the request path. Nothing in memory: the reference lives on the frame's temporaries stack, which
every normal exit and every landing block already releases. A `Core` member that suspends is covered
by the same hold, because the caller's hold lasts until the member returns.

**Declined:**
- **Retaining every property read.** It pays the pair on every `$this->x` operand, the most common
  operand in a method body, to cover the rare one followed by code that replaces it.
- **A fix written consumer by consumer.** Each consumer judging its own operands is how the gap
  opened, and it leaves the next consumer open.
