The dependency sweep re-runs a battery whose full order and conditional legs are the sweep
procedure's to state. Two of its rules are not negotiable, and this rule fixes them.

**A guard-test failure in `benches/abi-probe/` is never fixed by editing the threshold.** The guards
encode premises other decisions rest on — no native unwinding through JIT frames, what the sandbox
contains, a coroutine round trip's cost class, the regex engine's linear-time bound. If one fails
after a bump, the decision it names is what gets revisited, and the sweep stops until that is done.
The guards run on all three platforms, on any push that touches a crate whose cost they measure.

**A change to the dependency graph regenerates and commits `THIRD-PARTY-LICENSES.txt` in the same
commit.** CI fails otherwise, and correctly: the notice is a licence obligation, not a report
(`rule:testing/attribution-is-diffed-in-ci`). `cargo deny check` and the attribution generator's
`--check` run on every push that can change their answer, and unconditionally nightly and at release
(`rule:testing/ci-lanes`); they are what make a hold, a vendored fork and this rule enforceable
rather than aspirational.
