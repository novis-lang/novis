- **The one-second JWT case fails about one run in many, with `expired at N and it is now N` on the
  line that should verify.** Waiting for the second to turn before signing narrows that window
  without closing it, because `Core\Jwt::sign` reads the clock again itself. Re-run the case alone
  before believing a red conformance step; the fix is for it to assert against the `exp` the token
  carries rather than the second the spin saw. [until: reviewed 2026-09-16]
