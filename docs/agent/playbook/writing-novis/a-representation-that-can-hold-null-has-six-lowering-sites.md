- **A representation that can hold `null` has six lowering sites, not the four a `grep` for
  `Ty::Tagged` finds — and `==` against a written `null` is the one that looks covered and is not.**
  Five are where they look (`lower_coalesce`, `lower_isset_operand`, `truthy_convert`, `coerce`,
  `lower_binary`); the sixth is `lower_expr`'s own dispatch arm, which routes `$x == null` to
  `lower_null_identity` before `lower_binary` ever sees it whenever exactly one side is the literal,
  so a row added to `lower_binary` is dead for the spelling every test writes and a null value
  compares unequal to `null` while `isset` answers correctly one line above. When a construct has a
  fast path keyed on one operand being a literal, the fast path is a separate site a grep for the
  operator's own lowering will not find. [until: reviewed 2026-09-06]
