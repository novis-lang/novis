# Handoff

## State

**Goal `resource-ceilings`: stage 6's sweep now covers the two stores the goal never named.**
`crates/nvs-stdlib/src/bus.rs` holds `budget::Detached` at both ends — the publish allocates the
envelope inside one and the receiving core frees it inside another — which is sound because the
detached balance is per thread and **signed** (`crates/nvs-runtime/src/budget.rs:225`).
`crates/nvs-stdlib/src/topic.rs`'s per-core subscriber table brackets the name and the row it keys
and leaves the `Weak` prune outside on purpose. Both module docs carry the decision, two new guards
pin it, and `python tools/verify.py` is green.

**The driver's stage 3 check is a naming mismatch, not unlanded work, and that is why it repeats.**
The sampler and both poll halves are on disk: `crates/nvs-host/src/cpuclock.rs:193` spins rather
than sleeps and is the advances-with-work-not-with-waiting claim under another name, and
`crates/nvs-host/src/watchdog.rs:1117` stops the request past its ceiling and leaves the one under
it. The check's four drafted names are what has never run. This is the playbook's *a check can name
a test the tree already declares under a different name*, and it is the next group.

**One store stays unbracketed on purpose**, and it is still the compiled-pattern cache —
`crates/nvs-stdlib/src/regex.rs`'s gap 4 is the finding, waiting on M6's arena.

## Next group

**Stage 3: the clock, whose checks name tests the tree took other names for** — one file set,
`docs/agent/loop-goal.toml` with its copy `docs/agent/goals/40-resource-ceilings.toml`, reading
`crates/nvs-host/src/{cpuclock,watchdog}.rs` without editing them.
`rule:errors/on-limit` is what all four names are claims about.

- [ ] **Re-point the first check's names to the claims the tree landed** —
      `docs/agent/loop-goal.toml:8028` and `docs/agent/goals/40-resource-ceilings.toml:8021` are the
      two copies of the same list. `crates/nvs-host/src/cpuclock.rs:193` and
      `crates/nvs-host/src/watchdog.rs:1117` are the tests that already assert three of the four
      claims; rename the check rather than writing a second test of a claim already pinned.
- [ ] **Settle the fourth name before re-pointing it** — `a_platform_without_a_thread_clock_reports_no_cpu_ceiling_at_boot`
      asks for a report *at boot*, and `crates/nvs-host/src/cpuclock.rs:177` is only the fallback
      that answers `None`; `crates/nvs-host/src/watchdog.rs:973` is a case reading that answer, not
      a boot line. The goal's § *Standing decisions* is the specification — write the boot report or
      re-point the name, not both.
- [ ] **Check the stage's second list the same way, before touching it** —
      `docs/agent/loop-goal.toml:8034` names three `nvs-runtime` tests for the two poll sites, and
      whether each is drafted or landed is the same question asked of a different crate.

## Backlog

- `crates/nvs-stdlib/src/instance.rs:243` and `:368` leak a `ClassTable` charged to whichever request
  first constructs one — bounded and never freed, so nothing is credited, but the builder overpays.
- `crates/nvs-stdlib/src/channel.rs:233`'s parked-waiter `Vec` grows on one request's balance and
  never gives the buffer back — O(max concurrently parked), and the entries are the requests' own.
- `crates/nvs-stdlib/src/identity_store.rs:59` and `crates/nvs-stdlib/src/cli.rs:2804` were read this
  session and hold no cross-request heap; neither needs a bracket.
- The indented `thread_local!`s in `command`, `fatal`, `reflect`, `router`, `script`, `session` and
  `signal` are the unswept remainder of the store grep — most are inside a test module or a fn.
- A goal switch overwrites this file; what must outlive one goes in `docs/agent/carried-gaps.md`.
