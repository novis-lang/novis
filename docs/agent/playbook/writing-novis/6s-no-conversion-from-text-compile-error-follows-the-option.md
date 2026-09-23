- **§ 6's "no conversion from text" compile error follows the `#[Option]` marker rather than the
  type**, so a *positional* `#[Command]` parameter at a type nothing converts compiles and throws
  when it is run. That is why `ArgConv::Unconverted` stays reachable however many conversions are
  built. Keep the conformance case for that throw positional: rewritten with `#[Option]` it stops
  testing the arm it names. [until: gone crates/nvs-types/src/commands.rs:Unconverted]
