- **`a-full-channel-suspends-the-sender-rather-than-growing.nvst` can print `waits=6` against 7 under
  load.** A count of waits depends on scheduling, and it failed once in a full run on a machine out of
  page file, then passed alone six times. If it goes red on a loaded CI runner, make the case pin a bound
  on waits rather than one count. See `tests/conformance/core/a-full-channel-suspends-the-sender-rather-than-growing.nvst`. `until: gone tests/conformance/core/a-full-channel-suspends-the-sender-rather-than-growing.nvst`
