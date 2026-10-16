The accepted syntax is PCRE's, across both tiers, with three fixed points.

**There is no `u` modifier.** `rule:types/string-is-utf8` guarantees a `string` is UTF-8, so Unicode
mode is not optional and not a flag, and `.` is a code point. Matching over `bytes` is a separate,
explicitly byte-oriented entry point rather than the same members under a switch.

**A construct neither engine supports is a compile-time diagnostic naming it** — recursion,
subroutine calls and callouts among them. It is never silently ignored and never approximated,
because a behaviour difference the developer cannot see is precisely what the two-tier design exists
to avoid.

There is no `/e` modifier: a pattern never evaluates its replacement as code.
