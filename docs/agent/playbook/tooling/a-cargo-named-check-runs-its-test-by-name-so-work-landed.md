- **A `cargo-named` check runs its test *by name*, so a test landed, renamed or deleted under
  another name leaves the check reporting "did not run" rather than failing.** The live goal's
  record, `data/goals/<slug>.json`, carries the floor's checks beside its own, and a floor check is
  never edited to make something pass. Before you name, rename or delete a test, grep `data/goals/`
  for its name: a hit on a floor check means the check is re-pointed at the test that now holds the
  truth, and a hit on the live goal's own check alone means the test takes the check's name.
  [until: gone tools/nv/driver/accept.ts:cargo-named]
