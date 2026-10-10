- **A `Core` member cannot return a shape, and a registry row carries no nested `Qual` — two facts
  the `CoreTy` enum states only by omission.** There is no `CoreTy::Shape`:
  `crate::instance::SHAPE_ROSTER` is for values the engine builds, so a member whose spec answer is a record answers a `CoreTy::Instance`
  class with zero-argument members, as `Core\Regex\Match` and `Core\Process\Result` do. And
  `nvs_types::core_lib::qual_of` reads `Text`/`Blob` at the top level and inside a `Variadic` only,
  so `Array(&CoreTy::Text(Qual::Sink))` marks nothing; the ordinary argument check refuses a tainted element.
  [until: gone crates/nvs-stdlib/src/script.rs:crate::instance::SHAPE_ROSTER]
