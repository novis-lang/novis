- **A member on `SOURCE_MEMBERS` shifts its *ABI* arguments and not its written ones, so a call-site
  check still counts from zero.** `Core\Metrics::increment` carries `args: [4]` for three declared
  arguments because argument 0 is the call site, but that constant is spliced in
  `nvs_ir::lower::expr` long after `nvs_types::intrinsics` has run, so an `Intrinsic` row addressing
  the name writes `at: 0` and a row that paid for the shift would read the argument beside it. Read
  the shift where it is applied — the lowering — rather than off the helper's `args: [N]`, which is
  the only place the widened arity is visible.
  [until: gone crates/nvs-stdlib/src/registry.rs:SOURCE_MEMBERS]
