- **A `Value` that survives context teardown must be detached from the `Ctx`'s intrusive list, not
  just left alone.** A survivor still linked holds a `prev` naming the list head inside the freed
  context, so its last `unlink` writes into freed memory, and the symptom is a third party — a flaky
  test seeing NUL-overwritten bytes in a string that never allocated an object.
  `crates/nvs-runtime/src/object.rs`'s `Detach` guard is the fix; when a flake survives disabling
  the sweep body, suspect the list, and print the corrupted text rather than the status.
  [until: reviewed 2026-09-06]
