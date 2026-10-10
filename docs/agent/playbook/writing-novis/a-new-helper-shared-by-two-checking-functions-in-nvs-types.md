- **A new helper shared by two checking functions in `nvs-types` has no room for a seventh
  parameter: `expr`, `args`, `live`, `scope`, `ctx` and `env` are already six, and clippy's
  `too_many_arguments` fires at eight.** Extracting the common half of two checkers therefore fails
  the clippy leg the moment it adds one argument saying which caller it serves. Fold the discriminator into the data it selects over
  — an enum whose variants each hold the `&[TypeId]`, rather than an enum passed beside it — which
  keeps the count at seven. See `crates/nvs-types/src/expr/calls.rs`. [until: gone crates/nvs-types/src/expr/calls.rs:too_many_arguments]
