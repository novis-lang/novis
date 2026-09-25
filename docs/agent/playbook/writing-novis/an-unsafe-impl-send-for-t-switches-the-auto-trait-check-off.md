- **An `unsafe impl Send for T {}` switches the auto-trait check off, so the fields your safety
  argument rests on are never re-checked — including a dependency's.** `nvs_codegen::Unit`'s `Send`
  rests on `cranelift_jit::JITModule` being `Send` (the last `Arc` handle drops one on whatever core
  it lands on), and nothing about writing the impl makes the compiler agree; a cranelift bump could
  take it away and leave every test green. Pin each foreign clause with its own assertion —
  `const fn sends<T: Send>() {} sends::<JITModule>();` beside the type's own crossing test, and
  deliberately *not* the clause the argument does not need. [until: gone crates/nvs-codegen/src/lib.rs:cranelift_jit::JITModule]
