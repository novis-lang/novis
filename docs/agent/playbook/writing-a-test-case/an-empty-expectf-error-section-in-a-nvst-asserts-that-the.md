- **An empty `--EXPECTF-ERROR--` section in a `.nvst` asserts that the run *failed*, so a `--RUN-- test`
  case whose tests all pass reports `expected the run to fail, and it succeeded`.** The section is the
  stderr expectation, and its presence alone is what requires a non-zero status, so copying the shape
  from a neighbouring runner case that has failing tests carries that assertion along with it. Drop the
  section entirely when every `#[Test]` in the case passes, and keep it only where the run really ends
  non-zero. [until: reviewed 2026-09-20]
