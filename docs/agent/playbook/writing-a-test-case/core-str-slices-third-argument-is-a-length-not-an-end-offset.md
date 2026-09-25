- **`Core\Str::slice`'s third argument is a *length*, not an end offset, and getting it wrong still
  counts plausibly.** `Core\Str::slice($alphabet, $i, $i + 1)` is "from `$i`, take `$i + 1`
  characters", so a counted assertion still comes out right while row 16 hands the decoder a
  seventeen-character operand. Spell it `Core\Str::slice($s, $i, 1)`, and have a counting sweep echo
  the *set* it counted as well as the count. [until: gone crates/nvs-stdlib/src/str.rs:Core\Str::slice]
