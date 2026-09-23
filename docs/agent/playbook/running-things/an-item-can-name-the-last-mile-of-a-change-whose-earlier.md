- **An item can name the last mile of a change whose earlier mile never landed, and it reads the
  same either way.** "A proven call site emits no tag check" presumes the checker proved something,
  and a call through `callable(int): string` still answered `mixed` with its arguments unchecked —
  so the emission decision had nothing to read. Spend one call proving the premise before designing
  against it: `target/debug/nvs.exe run` over a three-line probe says what the compiler answers
  today, and the driver already built that binary at this commit. [until: reviewed 2026-09-07]
