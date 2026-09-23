- **A `.nvst` case's own helper has to be a `public static function` inside a class.** A plain
  `function render(...)` at file scope is `E0215: a function must be a method`
  (`rule:classes/no-free-functions-or-constants`), caught only when the case is run. Wrap it in a
  `final class` and call it `Render::pairs($m)`; a compiler-owned generic (`Core\ObjectMap<Tag,
  int>`) is accepted in that method's parameter list. [until: reviewed 2026-09-06]
