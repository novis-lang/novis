A generator object is not serializable, does not cross a `spawn`, `spawn worker` or `spawn script`
boundary, and does not `clone`. Each attempt is the same refusal any value that cannot be copied
soundly already gets.

The state-machine lowering (`rule:iteration/generators`) makes a generator an ordinary object with
ordinary fields, so the refusal is not a representation limit: those fields are a compiler-chosen
encoding of a suspended program point, and resuming that point in another isolate — or in a second
copy inside this one — has no meaning to give.

**Partly on disk.** The copy walk refuses a generator at every copy boundary
(`crates/nvs-runtime/src/graph.rs:394`), so storing one in `Core\Cache` throws
(`tests/conformance/core/cache-an-entry-is-a-copy-that-shares-nothing-with-the-request.nvst`).
`clone` is not refused: `clone` of a generator compiles and runs, because the checker refuses only
what cannot be an object (`crates/nvs-types/src/expr/mod.rs:837`) and `nvs_object_clone` copies any
object (`crates/nvs-runtime/src/object.rs:4009`).
