- **A spec signature can be unrepresentable in `CoreTy`, and the sibling roster entry has already
  resolved it.** § 16 wrote `Core\Signature::verify(...): array<string, mixed>` *and* called the
  result `tainted`, which cannot both hold — the qualifier is defined over `string` and `bytes`
  alone. Before transcribing a spec row whose **result** is qualified, read the neighbouring class's
  module doc: `crates/nvs-stdlib/src/jwt.rs`'s *a claim is text* section had the whole trade worked
  out already.
  [until: gone crates/nvs-stdlib/src/registry.rs:there is no `tainted array<T>` in `nvs_types`]
