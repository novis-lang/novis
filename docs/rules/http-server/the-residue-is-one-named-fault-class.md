What survives every other bound is **a memory-safety fault in `unsafe` code, or a miscompilation from the code generator**. Neither is containable in-process: `catch_unwind` does not catch a `SIGSEGV`, and wrong machine code has already done whatever it does before any check runs.

**A Rust panic is never part of the residue.** Every Rust entry compiled code calls with a context
runs inside `catch_unwind`: each helper through `run_helper`, including its memory pre-check and
collection, and the safepoint and stack-check slow paths through `abi::contain_panic`, including the
collection and the limit handler's runtime half. A panic there ends its one request with a `FATAL`,
and the next request on the core is answered. Code with no request beneath it is contained at the
task root (`rule:http-server/containment-does-not-end-at-the-helper`). The primitives that carry no
context, such as `nvs_array_set`, call no user code and have no wrap. An unwind that reaches a JIT
frame aborts the process, so a panic in one of them is a bug to fix, not a fault class to contain.

For these, tier D applies: the process dies, the supervisor restarts it, the requests in flight are lost, and recovery takes about a cold start — a figure that is, for this runtime, unusually small. A worker-process boundary under a supervisor was the alternative examined hardest and it does not pay: a refcount fault means one request reads another's memory, and N processes lower that probability without eliminating it; a deterministic miscompile on one route kills every process in turn; and the compiled-unit cache, the connection-pool sizing and the topic broadcast would all be duplicated or turned cross-process for a benefit that is narrow and partly theoretical. It is reopened when persistent connections ship, because a server holding many live connections that dies is a different event from a request server that dies.

The engineering response to this class is **not** a boundary; it is to reduce the rate at which the class occurs — the conformance cases run against generated code at every optimisation level, fuzzing the code generator the way the lexer and parser are fuzzed, and running the reference-counting paths under `tools/leak-check.sh`. A fault class that cannot be contained is one whose frequency is the only variable left.

Before any worker count above one is recommended, two numbers have to be measured: the per-unit size of
compiled code, which sets the duplicated cache's footprint, and the per-unit compilation time, which
sets the warm-up cost. A cold start bounds them jointly and separates neither.

**Declined:**
- **Process-per-request.** It is the only arrangement that contains a disclosure fault, and it discards
  the thread-per-core runtime the whole performance design rests on. It is the endpoint of the
  worker-process argument above, not a close call.
