- **A function's disassembly holds more cold blocks than its IR does, so "skip the `Propagate`
  blocks" does not skip the cold path.** Codegen invents blocks no `nvs_ir` block corresponds to —
  an ArithmeticError construction behind a `seto` check, the landing block's `Release` — and
  `rule:testing/debug-probes`'s probe calls are conditional, so a whole-section `call` count in
  `--dump-asm` prices three other mechanisms. What separates hot from cold in the VCode text is one
  shape, `testX` immediately followed by `jnz labelA; j labelB`, which is how every status word is
  checked: walk the `blockN:` graph from the entry and never follow that `jnz`.
  [until: gone crates/nvs-cli/src/main.rs:dump-asm]
