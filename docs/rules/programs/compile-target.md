Every compile target shares one front end, one resolver, one type checker and one IR. A target is a
*host context* plus a codegen backend, never a dialect: the type system, taint tracking, evaluation
semantics, name resolution, definite initialization and the checked-return ABI are identical wherever
a program is compiled.

Host state reaches a program through `Core` accessor classes rather than through language syntax, so a
target that has no inbound HTTP request simply does not offer `Core\Request`, and a call to an
accessor its context does not model is refused at exactly the point a spawned isolate's call to the
same accessor is. Where a target cannot provide a capability, using it is **a compile-time diagnostic
naming the target, never a silent no-op and never a weaker substitute** — a construct whose guarantee
quietly disappears on one target is worse than one plainly refused there.

The browser is not a target Novis builds. A `wasm32` backend for a browser tab is off the roadmap: its
standing cost is instruction selection and calling-convention lowering happening twice for every
codegen feature thereafter, and nothing is asking for it. The rule above governs the targets that
exist and any that is added later.
