- **A shape `type` alias that names itself does not compile, and the diagnostic is `array type nests
  past depth 32`.** `type Node = {value: int, next: ?Node};` expands through the alias until the
  depth bound stops it and reports once per field, so a linked list or a tree written as a shape
  reads as a depth problem in an annotation nobody nested. Write the recursive case as a class;
  `crates/nvs-types/src/lower.rs`'s `lower_type_at_depth` § *Known gaps* carries the wording.
  [until: gone crates/nvs-types/src/lower.rs:array type nests past depth]
