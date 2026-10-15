A generator object is not serializable, does not cross a `spawn`, `spawn worker` or `spawn script`
boundary, and does not `clone`. Each attempt is the same refusal any value that cannot be copied
soundly already gets.

The state-machine lowering (`rule:iteration/generators`) makes a generator an ordinary object with
ordinary fields, so the refusal is not a representation limit: those fields are a compiler-chosen
encoding of a suspended program point, and resuming that point in another isolate — or in a second
copy inside this one — has no meaning to give.

The copy walk refuses a generator at every copy boundary (`crates/nvs-runtime/src/graph.rs`,
`refusable`), so storing one in `Core\Cache` throws. `clone` refuses it twice over. The checker
refuses `clone` of a call whose declaration holds a `yield` with `E0858`
(`crates/nvs-types/src/expr/members.rs`, `reject_generator_clone`); that call is the only place a
generator is visible to it, because a generator types as `Iterator<T>` and a program's own classes
implement that too. Every other `clone` — through `mixed`, `object` or `Iterator<T>` — passes a guard
in front of the copy (`crates/nvs-runtime/src/object.rs`, `nvs_clone_guard`) that throws
`LogicError` with the copy walk's own message. The guard costs one helper call and one class load per
`clone`, and allocates nothing unless it throws.

A call that statically resolves to a generator is refused even where a subclass override is not one:
the written call names a generator, and refusing a sound program there costs less than a hole.
