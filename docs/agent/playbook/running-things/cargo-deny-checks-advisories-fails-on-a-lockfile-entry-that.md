- **`cargo deny check`'s `advisories` fails on a lockfile entry that is not yours.** The verdict is
  four lines — `advisories FAILED, bans ok, licenses ok, sources ok` — and the red one is `detected
  yanked crate (try 'cargo update -p chacha20')`, which arrives through `rand` and predates any
  dependency slice. A dependency slice owns `licenses`, `bans` and `sources`; read those three and
  say in the handoff that the fourth was already red, rather than fixing an unrelated lockfile entry
  or reporting your own change as the failure. [until: gone Cargo.lock:name = "chacha20"]
