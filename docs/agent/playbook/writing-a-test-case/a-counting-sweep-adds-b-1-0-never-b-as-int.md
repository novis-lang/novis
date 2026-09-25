- **A counting sweep adds `$b ? 1 : 0`, never `$b as int`.** `bool` converts to `string` and to
  `bool` alone (`E0708`, `rule:types/conversion`), so the *invariance over a sweep* shape spells its
  counter `$n = $n + (Core\Str::contains(…) ? 1 : 0);`. The rest of the shape works: an
  `array<Core\Time\Duration> $each = [0s, 1ns, …]` iterates with a typed `foreach` binding,
  `continue` skips a row a law does not apply to, and a bare `Core\X::member($arg);` is a legal
  statement when only the throw is wanted. [until: gone crates/nvs-diagnostics/src/lib.rs:E0708]
