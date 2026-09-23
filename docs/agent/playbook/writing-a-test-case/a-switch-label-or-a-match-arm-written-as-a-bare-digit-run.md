- **A `switch` label or a `match` arm written as a bare digit run never crosses representations, because
  it is lowered against the subject's own type.** `lower_switch` and `lower_match` pass
  `Some(subj_ty)` to `lower_expr`, so `case 2:` beside a `float` subject is a `ConstFloat` and a case
  meant to pin the cross-representation row pins the matched-pair one instead, silently. Write the
  other side as a binding — `uint $two = 2;` then `case $two:` — or as a literal no placement can move
  (`2.0` beside an `int`), and read the answer back rather than trusting the shape.
  [until: gone crates/nvs-ir/src/lower/control.rs:Some(subj_ty)]
