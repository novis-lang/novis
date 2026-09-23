- **A PHP warning printed to stdout makes an oracle leg unusable, so an `--ORACLE--` case never
  echoes a `NAN` or a `chr()` outside `0..255`.** PHP 8.4 and later warn when coercing `NAN` to a
  string and 8.5 deprecates `chr()` past 255, and the notice lands in the compared output. Otherwise
  a float may be echoed directly: Novis's rendering is PHP's, precision 14 with trailing zeros
  trimmed, so `sqrt(2.0)` prints `1.4142135623731` on both sides. [until: reviewed 2026-09-06]
