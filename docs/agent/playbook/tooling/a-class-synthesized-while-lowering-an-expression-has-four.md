- **A class synthesized while lowering an *expression* has four exits, not one.** `Lowering` builds
  one function, so a `crate::ir::Class` an expression invents (an anonymous function's environment, a shape
  literal's) rides out of `lower_method`/`lower_hook`/`lower_script` at their three identical
  `std::mem::take(&mut low.anon_fns)` sites *and* out of `lower_anon_fn`'s own recursion, or a body
  nested one level deeper contributes no class and codegen fails much later on a `New` naming a
  label the table lacks. Grep the take sites, not the struct field. [until: gone crates/nvs-ir/src/lower/mod.rs:crate::ir::Class]
