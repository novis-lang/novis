- **`NvsStr::from_raw`/`NvsArray::from_raw` return an *owning* handle.** Reading a refcount through
  one in a unit test releases a reference when it drops, so the test ends in a heap corruption
  rather than an assertion failure. Wrap it in `std::mem::ManuallyDrop`; `crate::arr::borrowed` is
  that wrapper for an argument, and `crate::instance::slot` is the borrowed read of an object's
  slot. [until: reviewed 2026-09-06]
