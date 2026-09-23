- **A value handed to an `rule:classes/property-hooks` `set` hook is *transferred*, so there is no
  "afterwards" to retain it in.** Every other assignment target keeps owning what was stored, so
  four of the five arms of `Lowering::lower_store` can retain after the store; a `set` hook is a
  call that hands the callee the reference, so a retain after it can read a value the hook already
  released, and only a valgrind fixture whose hook *discards* its argument shows it. `lower_store`'s
  `extra_owner` flag exists for that: the retain is emitted where each arm still holds a reference,
  which for that arm is *before* the call. [until: reviewed 2026-09-06]
