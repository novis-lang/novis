- **A `nvs-types` test that asserts an interned type's `describe` string is fragile.** A union
  orders its members by type id, so registering a member anywhere can flip `T|null` to `null|T`.
  Compare against `interner.make_union([...])` instead. [until: reviewed 2026-09-06]
