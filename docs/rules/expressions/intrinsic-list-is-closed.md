The compiler recognises intrinsics by fully-qualified name from a **closed set in its own source**
(`crates/nvs-types/src/intrinsics.rs`). There is no attribute, no manifest field and no configuration
by which a `Core` class, a subsystem or an extension declares itself intrinsic, and there is no way
for user code to add a row.

Adding an entry is a compiler change with a fixture. Each row names its owner, the grammar its
literal argument is read against, and what preparation produces — so the roster and the validation
that runs over it stay one artefact rather than two lists to keep in step.

A closed set is what makes `rule:expressions/preparation-preserves-behaviour` enforceable: every row
can be tested against its runtime twin, because the rows are enumerable.
