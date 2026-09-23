- **A `Core` symbol that is not a member breaks
  `every_registered_member_has_an_implementation_address`.** `nvs_stdlib::symbols()` used to be
  exactly one entry per `CLASSES` member and that test asserts the count, so a constructor symbol
  from `registry::CONSTRUCTORS`, or anything else chained in beside the members, has to be added to
  the test's right-hand sum in the same edit. The failure is a bare `left: N, right: M` in `-p
  nvs-stdlib --lib`, naming no symbol. [until: reviewed 2026-09-06]
