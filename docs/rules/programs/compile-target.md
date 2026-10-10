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
exist and any that is added later. Wasm as an extension host (`rule:packaging/an-extension-is-a-sandboxed-wasm-component`)
runs in the opposite direction and is not this target.

Such a target would drop three things, each a diagnostic: `spawn`, because a tab has no OS thread or
process substrate and no shared compiled-code cache; suspension, because native stack-switching has no
standard `wasm32` equivalent; and `.nvsx` extensions, because no browser ships a Component Model host.
What would reopen it is an embedding that wants Novis in a browser tab, and only that. A cheap, widely
shipped browser stack-switching mechanism would remove the suspension exclusion, and the Component
Model shipping in browsers the extension one, but neither removes the second backend's cost.

**Declined:**
- **Compiling the native JIT itself to wasm and JIT-ing Novis source at page load.** A wasm module
  cannot mark its own memory executable, so no browser permits it.
- **Asyncify, to keep one execution model on every target.** It instruments every function reachable
  from a suspend point, not only the ones that suspend, which is a whole-module cost paid by default.
- **A weaker isolation primitive for `spawn` on a target that lacks the substrate**, such as a silent
  same-thread call. That is the weaker substitute the rule above forbids.
- **Modelling a new host context as `Core\Request` or `Core\Cli` with methods added.** It makes "is this
  call legal in this context" ambiguous again; a new context gets its own accessor class.
