- **A `#[Core\Json\Derive]` class with no constructor compiles and then has no codec at run time**, and
  the `LogicError` blames the one thing that is not wrong — the attribute the class plainly carries.
  `rule:core-classes/derive-field-list` makes a decode an ordinary `new`, so with no parameter list the
  field list is empty, and `crates/nvs-types/src/derive.rs:1878` stays quiet on the assumption
  `ctor_init` already reported it, which it does not when every property has a default. Give the class
  a constructor taking every field. [until: gone crates/nvs-types/src/derive.rs:ctor_init]
