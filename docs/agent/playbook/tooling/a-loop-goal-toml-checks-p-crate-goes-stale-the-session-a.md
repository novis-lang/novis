- **A goal record's check's `-p <crate>` goes stale the session a module moves between crates,
  and the failure then reads exactly like unwritten work.** Stage 2's event framing moved from
  `nvs-server` to `nvs-runtime` so `nvs-stdlib` could name it without closing a cycle, and its three
  checks went on naming `-p nvs-server` in their `args` — which reports all nine tests as "did not
  run" while every one of them passes one crate over. When a `cargo-named` check reports its *whole*
  list missing, grep the test names across `crates/` before writing anything: a name that resolves
  somewhere else is a check to re-point, never a test to write again. [until: gone tools/nv/cmd/orient.ts:const TRIAGE]
