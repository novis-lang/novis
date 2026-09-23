- **A `Core` member cannot return a shape, and a registry row carries no nested `Qual` — two facts
  the `CoreTy` enum states only by omission.** There is no `CoreTy::Shape`:
  `crate::instance::SHAPE_ROSTER` is for values the engine builds (`Core\Issue`,
  `Core\Script\Result`), so a member whose spec answer is a record answers a `CoreTy::Instance`
  class with zero-argument members, as `Core\Regex\Match` and `Core\Process\Result` do. And
  `nvs_types::core_lib::qual_of` reads `Text`/`Blob` at the top level and inside a `Variadic` only,
  so `Array(&CoreTy::Text(Qual::Sink))` marks nothing — what refuses a tainted element is the
  ordinary argument check, because `array<tainted string>` is not `array<string>`.
  [until: reviewed 2026-09-06]
