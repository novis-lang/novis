- **An object cannot cross an isolate boundary outward unless it crossed inward first.**
  `Live::admit` compares descriptor addresses, so a `class Node` declared in both parent and child
  is two descriptors and `return new Node(...)` is `ok=false`. Pass the object in as `args:` and
  have the child hand it back, as `examples/cycles.nvs` does.
  [until: gone crates/nvs-runtime/src/graph.rs:on the receiving side is a different class]
