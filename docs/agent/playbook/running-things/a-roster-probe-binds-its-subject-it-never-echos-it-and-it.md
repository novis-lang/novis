- **A roster probe binds its subject; it never `echo`s it, and it gets one panic per run.** An `echo
  $subject` line goes through `concat_operand` first, so the probe reports that function's missing
  row rather than the operator's; `mixed $x = $a & $a;` asks the question meant. The checker reports
  every diagnostic in a file at once while lowering panics on the first shape that gets that far, so
  put the shapes you expect refused in one file and the ones you expect to lower in another.
  [until: gone crates/nvs-ir/src/lower/expr.rs:concat_operand]
