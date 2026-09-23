- **An enum is not spelled the way PHP spells it: bare names, commas, no `case` keyword.** `enum
  Mode: int { case Read = 1; }` parses as a class and cascades `E0220` *"an enum declares only cases
  and an optional backing type"* at the `{` then `E0101` *"expected a class member"* per line. Write
  `enum Mode { Read = 1, Write = 2 }` (`enum Mask: uint { … }` for a `uint` backing, since a case
  past `int` is `E0437`). [until: reviewed 2026-09-06]
