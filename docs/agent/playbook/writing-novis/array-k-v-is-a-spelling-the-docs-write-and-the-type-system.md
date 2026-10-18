- **`array<K, V>` is a spelling the docs write and the type system has no form for.**
  `nvs_types::ty::Ty::Array` carries one `TypeId` and there is no `CoreTy` for a keyed array, so `array<string, mixed>` in a doc
  is declared as `CoreTy::Array(&CoreTy::Mixed)` and the key rule is enforced at the call site where
  it can be. Same family: `Core\Arr::append`'s second parameter is one element, so
  `Core\Arr::append($a, $b)` over two arrays is `E0401: expected int, found array<int>`; build the
  combined array with a `foreach` and one `append` per element. See `crates/nvs-types/src/ty.rs`.
  `until: gone crates/nvs-types/src/ty.rs:Array(TypeId)`
