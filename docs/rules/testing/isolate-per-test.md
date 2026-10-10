Each test runs on an isolate of its own: it shares compiled code with its siblings and nothing
else — not a static, not a cache entry, not an open handle. A static one test increments reads its
declared initial value in the next, always, with no flag to change it
(`rule:statements/an-isolate-has-its-own-statics`).

This is not a hardening measure bolted onto a sequential runner; it is what makes a parallel one
possible at all, because an isolate is cheap enough to be the default rather than a per-case escape
hatch. Two consequences follow and are answered elsewhere: shared expensive setup needs its own
mechanism (`rule:testing/fixtures`), and execution order carries no meaning, so the report is in
declaration order regardless of what finished when (`rule:testing/runner-is-strict`).

Each test isolate spends its parent's budget, so a suite has the same enforceable ceiling on memory
and CPU that a request has, and a runaway test is terminated rather than left to consume the
machine.

- **Memory:** each test in flight holds one isolate, one copy of each fixture it declares and one
  ledger entry per assertion it executes, all released when it joins. That is O(tests running), not
  O(tests in the suite): a suite of any size costs at one instant what its running tests cost.

**Declined:**
- **One isolate per test class.** It is cheaper and familiar, and it restores exactly the ordering
  dependence between cases that makes real suites flaky. Fixtures recover the setup cost it would
  save without the coupling (`rule:testing/fixtures`).
