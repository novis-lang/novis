- **Deleting a `Core`-only parameter variant deletes the *bound*, not just the binding, and the
  hole is silent.** `CoreTy::CallableShapeTo` looked like pure binding machinery, so replacing it
  with `CoreTy::Var("S")` plus a walk reads as a clean simplification — but `S` binds to anything,
  so `Core\Task::all(5)` then type-checks and answers `mixed`, and only the runtime notices.
  Before retiring a bespoke parameter spelling, ask what refuses the argument once it is gone: if
  the answer is "a type variable", the variant was a bound and Novis has no bounded type variable
  to replace it with. [until: gone crates/nvs-stdlib/src/registry.rs:ShapeOfCallables]
