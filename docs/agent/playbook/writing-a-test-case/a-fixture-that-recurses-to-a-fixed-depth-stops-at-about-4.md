- **A fixture that recurses to a fixed depth stops at about 4,400 calls.** A task's stack is 1 MiB,
  and the unwinding reserve takes 256 KiB of it (`crates/nvs-host/src/stack.rs:88`); an ordinary
  static method with one local throws `RecursionError` between 4,400 and 5,200 calls, debug and
  release alike, and a fatter body stops sooner. Keep a proof that must succeed under about 2,000,
  and let one that must fail recurse with no base case at all.
  `until: gone crates/nvs-runtime/src/throwable.rs:RecursionError`
