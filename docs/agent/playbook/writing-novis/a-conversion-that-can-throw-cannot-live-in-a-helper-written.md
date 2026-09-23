- **A conversion that can throw cannot live in a helper written for infallible rows — it needs the
  frame's landing block.** `nvs_ir::lower::Lowering::coerce` performs `rule:types/conversion`'s
  implicit `int`/`uint` → `float` widening, which throws above 2^53, so `&Env` had to be threaded
  through every one of its call sites and `close_nullsafe` before the row could land. Before adding
  a fallible row to `coerce` or any helper like it, check that it is handed the landing block at
  all. [until: reviewed 2026-09-06]
