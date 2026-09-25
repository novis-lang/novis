- **An inline array literal is `array<mixed>`, so a typed `foreach` binding or a generic `T` bound
  from one fails.** `rule:types/conversion` leaves an untyped literal unplaced, so the `foreach`
  binding is `E0401` at the binding and `string $one = Core\Cli::select("q", ["a", "b"])` is `E0401:
  expected string, found mixed` because `crate::generics::bind` reads `array<mixed>`. Name the
  array's type where it is built (`array<string> $choices = [...]`), not where it is walked or
  passed. [until: gone crates/nvs-types/src/generics.rs:fn bind]
