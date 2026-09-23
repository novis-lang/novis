- **An `ir::Class` field typed with an `nvs_types` type fails to compile in `nvs-codegen`.** That
  crate lists `nvs-types` under `[dev-dependencies]` only, so the lib half sees `nvs-ir` and
  `nvs-runtime` and nothing above them, and the error reads as a missing dependency rather than a
  deliberate boundary. Re-export the type from `crates/nvs-ir/src/lib.rs` and name it `nvs_ir::…`
  in codegen. [until: reviewed 2026-09-17]
