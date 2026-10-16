The compiler recognises intrinsics by fully-qualified name from a **closed set in its own source**
(`crates/nvs-types/src/intrinsics.rs`). There is no attribute, no manifest field and no configuration
by which a `Core` class, a subsystem or an extension declares itself intrinsic, and there is no way
for user code to add a row.

Adding an entry is a compiler change with a fixture. Each row names its owner, the grammar its
literal argument is read against, and what preparation produces — so the roster and the validation
that runs over it stay one artefact rather than two lists to keep in step.

A closed set is what makes `rule:expressions/preparation-preserves-behaviour` enforceable: every row
can be tested against its runtime twin, because the rows are enumerable.

The roster is a permanent coupling between the compiler and the library: the compiler knows a
handful of `Core` method names. The closed set bounds that coupling, since it grows only when a row is
added deliberately. What it costs is that a package or an extension cannot have its own
pattern-shaped argument checked early.

**Declined:**
- **An open intrinsic mechanism**, an attribute or manifest field by which any library declares a
  method foldable. It turns "the compiler knows a few names" into "the compiler runs code a library
  chose while compiling", which is a larger attack surface and much harder to reason about.
