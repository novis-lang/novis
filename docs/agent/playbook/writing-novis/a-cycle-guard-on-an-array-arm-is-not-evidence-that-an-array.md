- **A cycle guard on an array arm is not evidence that an array can cycle.** An array is
  copy-on-write, so `$a[] = $a` appends a copy and an array-only descent is finite with no guard at
  all (`crates/nvs-runtime/src/graph.rs:20`); `crates/nvs-stdlib/src/json.rs:817` guards its array arm
  only because an **object** inside that array puts the same allocation back on the path. Audit a
  walker by asking whether it ever descends into an object, not whether it descends into a container.
  [until: gone crates/nvs-runtime/src/graph.rs:copy-on-write storage]
