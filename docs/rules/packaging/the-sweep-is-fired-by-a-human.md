Being current is a standing obligation, so it has a schedule. The schedule is a **prompt the user
runs**, and **no agent, loop, cron or CI job ever starts it on its own** — an automated bump would
commit the one class of change whose whole point is that a person weighed it, and would do so on
the day a release is being cut. Automated dependency pull requests are refused for the same reason:
green CI does not decide a change's classification.

| When | What moves |
|---|---|
| Weekly | `cargo update` within the declared ranges — one commit, one battery run |
| Monthly | Semver-incompatible crate bumps, CI action majors, the developer toolchain, and every hold whose date has passed |
| Within one release cycle of a Rust stable release | The pinned toolchain, in its own commit, green on all three platforms |
| Immediately, ahead of any other work | A RUSTSEC advisory or a yanked crate in the tree — the one item that may interrupt a milestone |
| At each release | The whole battery, the attribution file and the callgrind figures |
| At a major only | The Rust edition, and any hold that turned out to be permanent |

**A dependency more than one minor behind for over 30 days without a hold record is stale, and stale
is a defect**: it gets a hold with a date and a reason, or it gets updated. Being behind is never a
neutral state, because the alternative is one enormous forced migration under a security deadline.
The cadence applies in both of `rule:packaging/the-version-contract-starts-at-0-1-0`'s regimes.
