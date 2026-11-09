The dependency sweep re-runs a battery whose full order and conditional legs are the sweep
procedure's to state. Two of its rules are not negotiable, and this rule fixes them.

**A guard-test failure in `benches/abi-probe/` is never fixed by editing the threshold.** The guards
encode premises other decisions rest on — no native unwinding through JIT frames, what the sandbox
contains, a coroutine round trip's cost class, the regex engine's linear-time bound. If one fails
after a bump, the decision it names is what gets revisited, and the sweep stops until that is done.
The guards run on all three platforms in every CI run.

**A change to the dependency graph regenerates and commits `THIRD-PARTY-LICENSES.txt` in the same
commit.** CI fails otherwise, and correctly: the notice is a licence obligation, not a report
(`rule:testing/attribution-is-diffed-in-ci`). `cargo deny check` and the attribution generator's
`--check` run in every CI run, including the nightly and the release's
(`rule:testing/every-run-runs-every-job`); they are what make a hold, a vendored fork and this rule enforceable
rather than aspirational.

**`cargo deny` reads every lockfile in the tree, not the root's alone.** The root `Cargo.lock` gets
the whole check: advisories, licences, bans and sources. The eight others get `check advisories`
under the root's `deny.toml`: `extensions/image`, `extensions/intl`, `extensions/pdf`,
`extensions/spreadsheet`, `fuzz`, `tools/covwrap`, `tools/nv-scan` and
`website/guides/highlight-extension`. Licences stay root-only, because what the binary ships is
`gen-attribution`'s to judge. CI's `supply-chain` job and `bun nv ci --check` run all nine, and
`LOCKFILE_DIRS` in `tools/nv/cmd/ci.ts` is the list's home. A lockfile CI does not read can
carry a live advisory nobody sees. Cost: one advisory pass per lockfile, a few
seconds each.
