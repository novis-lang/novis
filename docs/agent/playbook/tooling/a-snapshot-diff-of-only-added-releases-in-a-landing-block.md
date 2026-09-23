- **A snapshot diff of only *added releases in a landing block* can still be a double release.** The
  forget that takes a transferred argument off `Lowering::owned_temporaries` has to run **before**
  `emit_fallible`, which builds the call's fault edge out of whatever is on the stack at that
  moment, and a callee releases its parameters on its *throwing* edge as much as on its normal one.
  Ask which instruction the changed `bbN` is the `! bb` of: if it is the call that consumed the
  value, the release does not belong there. [until: reviewed 2026-09-06]
