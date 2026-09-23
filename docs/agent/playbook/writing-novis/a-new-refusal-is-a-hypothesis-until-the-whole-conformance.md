- **A new refusal is a hypothesis until the whole conformance tree has run it, and the tree is where
  the counterexample lives.** `E0787`'s first shape — refuse every write to a property with a `get`
  hook and no `set` — passed its own new case and was refuted by
  `lang/the-inout-marker-is-required-at-both-ends-and-replaces-every-ampersand.nvst`, whose class
  arms its own `get`-only property from its constructor, turning the rule scope-shaped rather than
  blanket. Write the refusal, run `./target/debug/nvs.exe test tests/conformance`, then write the
  case that pins it — a case written first only pins the rule you already believed.
  [until: reviewed 2026-09-06]
