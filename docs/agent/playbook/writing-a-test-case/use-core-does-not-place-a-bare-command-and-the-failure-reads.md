- **`use Core;` does not place a bare `#[Command]`, and the failure reads as if the roster edit did
  not land.** A `use` aliases one *name*, so with only `use Core;` in scope the attribute resolves
  to `\Command` and the answer is `E0303: `Command` is not declared` — the ordinary undeclared-name
  refusal, pointing at the wrong file. Import it as `use Core\Command;` exactly as `#[Test]`'s is
  `use Core\Test;`, or write `#[\Core\Command(...)]` fully qualified when the case is about the
  match rather than the import. [until: reviewed 2026-09-06]
