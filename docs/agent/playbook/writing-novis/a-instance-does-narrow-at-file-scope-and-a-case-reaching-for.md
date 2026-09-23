- **A `?Instance` does narrow at file scope, and a case reaching for a `?Match` needs no helper
  class.** `var $found = Core\Regex::match($s, $p); if ($found == null) { … } else { … }` compiles,
  and inside the `else` the receiver is the class type. The neighbouring trap — a `?array<T>` that
  cannot be indexed even after a `!= null` guard — is about the element type, so do not wrap every
  nullable in a `public static function`. [until: reviewed 2026-09-06]
