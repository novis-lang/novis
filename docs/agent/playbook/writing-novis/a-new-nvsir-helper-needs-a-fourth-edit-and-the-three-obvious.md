- **A new `nvs_ir::Helper` needs a fourth edit, and the three obvious ones build without it.** The
  variant, `nvs_ir::print`'s name and `nvs_codegen::emit`'s `helper_symbol` row all compile; what
  fails is `cranelift-jit` panicking at run time with `can't resolve symbol nvs_value_add` and no
  Novis frame in the message, because a `#[no_mangle]` helper is found in
  `nvs_runtime::helpers::symbols()`'s `(name, address)` table, never by name in the host process.
  `grep -n "nvs_call_callable" crates/` names all four sites of a neighbouring helper at once.
  [until: gone crates/nvs-runtime/src/helpers.rs:fn symbols]
