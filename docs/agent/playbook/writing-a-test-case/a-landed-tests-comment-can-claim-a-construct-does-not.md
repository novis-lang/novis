- **A landed test's comment can claim a construct does not compile, and be out of date.**
  `time-datetime-startof-and-endof-are-one-agreement-over-every-unit.nvst` said an
  `array<Core\Unit>` element and a helper's own `Core\Unit` parameter were both `E0401`,
  and both compile today, so an example written from that comment would have avoided the
  two shapes a reader actually writes. Probe the claim with a five-line program through
  `target/debug/nvs.exe run` before believing it, and rewrite the comment in the same
  session. [until: reviewed 2026-09-19]
