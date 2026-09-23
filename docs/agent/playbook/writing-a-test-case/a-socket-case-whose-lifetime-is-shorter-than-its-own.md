- **A socket case whose lifetime is shorter than its own handshake flakes under the whole test
  run.** The clock `opened` starts runs from before the real loopback upgrade, so a sub-second
  `maxDuration` can expire before the talking peer lands a frame, and the case then fails its *was
  it talking* assertion rather than its bound. Size such a lifetime above the connection it also
  covers, and lengthen the peer's script rather than tightening the bound. [until: reviewed 2026-09-17]
