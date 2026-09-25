- **A `.nvst` case cannot annotate a `var` and cannot declare a bare function, so naming a `Core`
  class takes a static method's parameter list.** `var $msg: ?Core\Socket\Message = …` is a run of
  `E0101`/`E0102`s and a top-level `function describe(…)` is `E0215`
  (`rule:classes/no-free-functions-or-constants`). Name the class in a `public static function`'s
  parameter list inside a `class` block, where `Attribution::holders` reads it.
  [until: gone crates/nvs-diagnostics/src/lib.rs:rule:classes/no-free-functions-or-constants]
