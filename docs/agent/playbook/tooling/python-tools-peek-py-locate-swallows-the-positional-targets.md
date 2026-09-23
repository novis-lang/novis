- **`python tools/peek.py --locate` swallows the positional targets in the same call and says
  nothing about it.** A call written `peek.py sse.rs:362-448 --locate Foo Bar` prints the `file:line`
  anchors alone, so the window you asked for comes back missing and reads as though the file had
  nothing at that range. Send `--locate` as a call of its own and put the windows in the next one.
  [until: reviewed 2026-10-11]
