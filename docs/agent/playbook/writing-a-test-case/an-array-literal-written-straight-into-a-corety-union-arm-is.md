- **An array literal written straight into a `CoreTy::Union` arm is typed before the arm is
  considered, so `["a", "b"]` against `string|array<string>` is refused as `array<mixed>`.** The
  checker places an enum-case or a scalar literal inside a union, but an array literal infers its own
  element type first and then fails to assign, which reads as the option's type being wrong rather
  than the literal's. Bind it to a typed variable one line up — `array<string> $lines = ["a", "b"];`
  — and pass that, which assigns cleanly and documents the arm at the call site.
  [until: gone crates/nvs-types/src/core_lib.rs:CoreTy::Union]
