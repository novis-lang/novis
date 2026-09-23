- **A hostile case that nests source "as deeply as the parser allows" is refused well short of 96
  levels, and the refusal reads as the attack having worked.** One `if (1) { … }` costs the parser
  several of its 96 levels, so `Core\Str::repeat('if (1) { ', 90)` throws a `ParseError` while 40
  parses and walks, and a case written to reach a deep tree then asserts nothing about depth.
  Probe the depth with `target/debug/nvs.exe run` on a throwaway program before writing the case,
  and put the level that parses in the comment beside the number. [until: reviewed 2026-09-20]
