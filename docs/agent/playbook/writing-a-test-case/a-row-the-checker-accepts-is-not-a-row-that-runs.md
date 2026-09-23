- **A row the checker accepts is not a row that runs.** `nvs-codegen` refuses with *"does not lower
  a binary operator over mismatched representations"*; `rule:types/arithmetic`'s promotions and
  `E0715` are out of that hole, so what reaches it is mostly a `Ty::Tagged` operand no diagnostic
  names. Run the rows in a scratch `.agent-tmp/*.nvs` before writing a case off a rule's compiling
  rows, and pin one that does not lower in the crate's `tests/`. [until: reviewed 2026-09-06]
