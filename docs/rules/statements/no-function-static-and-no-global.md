Two spellings are refused, and each diagnostic names its replacement:

- `static int $calls = 0;` in a function — *function-scope `static` is not supported; declare a
  `private static` property on a class, or pass the value as a parameter*.
- `global $x;` — *`global` is not supported; pass `$x` as a parameter, or make it a `static` property or a
  `const`*.

A diagnostic that says only "not supported" is a bug against this rule, not a faithful implementation of
it.

A function static is the one binding whose definite assignment cannot be checked: its initialiser runs on
the first call and on no later one, so on every call after the first the binding is live while its
initialiser is not on the executed path. It is also a third storage class for one keyword — a per-function
slot, per isolate, with a run-once flag the JIT cannot fold away — and it hides from the signature that a
function's result depends on how often it has been called. A function static could never outlive its
request in any case, so nothing a program could rely on across requests is lost.

Without it, an isolate has one table of statics, the class statics, with one lifetime and one teardown
path, so a fresh isolate is fresh by building that one table again. Every use of a function static has a
better-placed replacement with a name, a type, a visibility and a home: a cache is a `private static
array<T>` on the class that owns it, where it can also be cleared and inspected; a one-time
initialisation is a static property or a constant; a call counter is a static property. The rewrite is
not always local: a free function in a procedural file needs a class to hang the property on, or a
parameter to carry the state.

`global` hides a function's inputs from its signature in the same way, and a `const` already covers
everything a read-only global would be used for.

**Declined:**
- **A function static with a type slot.** It would put the one exception into the definite-assignment
  analysis that the mandatory annotations pay for (`rule:types/declaration`).
- **A function static whose initialiser runs when the isolate starts.** A side-effecting initialiser
  would run at a different time than the code says, and the second table of statics would remain.
- **`global` for reading only.** The function's inputs are still missing from its signature, and a
  `const` reaches every read-only value the declared way.
- **A `#[Memoize]` attribute as the replacement for a cache.** A caching construct has questions of
  its own (how a key is derived, how long an entry lives, how large the cache may grow), and it is a
  feature with its own rule if it ever exists, not part of this one.
