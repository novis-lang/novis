- **A `type` alias naming another `type` alias is refused, and the diagnostic calls it a class.**
  `E0307` reads the written atom rather than what it resolves to
  (`crates/nvs-hir/src/resolve.rs:284`), so `type B = A;` over `type A = int;` is "a `type` alias may
  not name a single class, interface or enum on its own" — right about the shape, wrong about the
  kind. Chain through a wrapper instead, `type B = ?A;` or `type B = array<A>;`, which is what a
  hostile case needs to build a long chain that compiles at all. [until: reviewed 2026-09-19]
