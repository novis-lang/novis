- **A fixture that needs a route table does not need an `autoload` root.** The table is collected
  over every declaration the program checks, so a class declared in the entry file lands in it —
  `crates/nvs-cli/tests/fixtures/api/base.nvs` is one file with a class and an `echo`, and `nvs
  build --openapi` emits both its operations. `examples/routes.nvs` splits across a root to
  demonstrate `rule:packaging/autoload-probes-fold-into-the-cache-key`'s scan, not because an
  emitter fixture has to. [until: gone crates/nvs-cli/tests/fixtures/api/base.nvs:autoload]
