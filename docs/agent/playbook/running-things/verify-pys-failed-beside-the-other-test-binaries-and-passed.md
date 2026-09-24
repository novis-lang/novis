- **`nv verify`'s "failed beside the other test binaries and passed alone" can be a hash-order
  flake rather than a shared resource.** A test comparing two values through `format!("{:?}")`
  prints any `HashSet` field in an order that differs per process, so it fails at random and passes
  the rerun the tool does to check — which reads exactly like a shared port or temp path. Read the
  two sides of the assertion first: the same elements in a different order means the `Debug` impl is
  what needs fixing, not the isolation. [until: reviewed 2026-09-17]
