- **A sweep over code points writes `Core\Str::fromCodePoint($c as uint)`, and orders two strings
  with `Core\Str::compare`.** `Core\Str::fromCodePoints` takes `array<uint>` while `Core\Arr::range`
  answers `array<int>`, and `array<int> as array<uint>` is a conversion `rule:types/conversion`
  still owes; `<`/`>` over two `string`s does not lower. `Core\Str::compare($a, $b) <= 0` is an
  `int` comparison, and over fixed-width zero-padded hex it is the numeric one, so a case can assert
  that a `Core\Uuid::v7` sweep never goes backwards. [until: reviewed 2026-09-06]
