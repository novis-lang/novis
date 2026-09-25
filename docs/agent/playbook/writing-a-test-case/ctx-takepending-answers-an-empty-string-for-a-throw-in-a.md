- **`Ctx::take_pending` answers an empty string for a *throw* in a hand-built context, so a
  bare-harness test cannot assert on a thrown message.** `Ctx::buffered()` installs no runtime error
  class and `Thrown::message()` returns `String::new()` when its object is null, so the member seems
  to have said nothing. Assert the `rule:errors/propagation` *status* (`THROWN` against `FATAL`);
  `benches/abi-probe/tests/invariants.rs`'s `decode_on_this_stack` is the shape.
  [until: gone benches/abi-probe/tests/invariants.rs:decode_on_this_stack]
