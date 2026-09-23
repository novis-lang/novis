- **`nvs-codegen` has two catch-alls under one operator, and `Ty::Bool` passes the cheap-looking
  gate.** `emit_binop`'s representation gate near the top is `matches!(ty, Ty::Int | Ty::Uint |
  Ty::Bool)`, so an unrowed `bool` pair never reaches either message: `true + true` lowered to an
  `iadd` over the `i8` and printed a number, where the same hole over two `string`s merely refused.
  When closing an operator table at the checker, probe the `bool` row first and read its answer
  rather than its exit status — a refusal you can see is the good case. [until: reviewed 2026-09-06]
