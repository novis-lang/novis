- **A new `panic!` naming a shape in `nvs-ir` fails `cargo test --test refusals` on a count, not on
  the panic.** `tools/nv/cmd/holes.ts` reads a refusal site off the construct rather than off the
  wording, so a "this row is not lowered yet" panic trips `CEILING` even when the attribution half of
  that gate is green, and rewording one to dodge the recognizer is the move that file forbids by
  name. Land the whole shape, close another site in the same slice, or raise `CEILING` and name in
  its doc comment the item that brings it back down. [until: gone tools/nv/cmd/holes.ts:panic!]
