- **A named class does not satisfy a shape type, whatever its properties are called.** `View::y(new
  Point(3, 4))` against a `{y: int}` parameter is `E0401: expected {y: int}, found Point`, because
  `rule:types/shape-type`'s width subtyping is shape-to-shape only (`nvs_types::expr::assign`). The
  only widening a case about a shape receiver can write is a narrower shape; do not design one
  around the class direction. [until: reviewed 2026-09-06]
