# Handoff

## State

**Goal `finish-response` — stages 2 and 3's refusal are green, and stage 4's `-p nvs-stdlib` check is
closed.** `Core\Script::finish()` inside a `Core\Task` child is pinned in
`crates/nvs-stdlib/src/script.rs:1388`: the marker lands on the child's own pending slot, the request's
queue is still undrained and its hooks still registered, and the request's own ordinary end is what the
report names. The child is sealed besides, which is `nvs_runtime::deferred`'s *only the request's own
task may register* read for this ending.

**`E0814` is the arm refusal**, `nvs_diagnostics::code::E_CATCH_ARM_NAMES_THE_FINISH_MARKER`, reported
by `reject_finish_marker_arm` (`crates/nvs-types/src/expr/members.rs:479`) from both call sites — the
block form at `crates/nvs-types/src/locals.rs:1290` and the expression arm at
`crates/nvs-types/src/expr/mod.rs:864`. A clause's type is now lowered whether or not it binds, because
a clause that binds nothing names a class just as loudly and nothing lowered one before. A clause
naming the marker *and* another class is `E0245` before it is this.

**A `throw` of the marker needed nothing**: it descends from `Throwable` not at all, so `E0780` already
reports it as an operand outside the tree, and `crates/nvs-types/tests/finish_marker.rs` pins both ends
in one file so the day the marker is given a parent both fail together.

**Stage 3's unwind is still red** — the six `nvs-codegen` tests below — and stage 4 still owes three
`.nvst` cases and `examples/finish.nvs`. The `-p nvs-host` half of stage 4 was not looked at this
session.

## Next group

**Stage 3: the unwind, in the codegen backend** — one file set,
`crates/nvs-codegen/tests/throwing.rs`, whose `a_frame_that_throws_releases_the_strings_it_still_held`
and `a_fatal_releases_the_frames_locals` are the shapes both items below are written against.

- [ ] **Write the three `a_finish_…` tests of stage 3's first check** — that every `finally` between
      the call and the root runs, innermost first, three frames deep — at
      `crates/nvs-codegen/tests/throwing.rs:326`. The lowering already seals the block with a
      `Terminator::Throw` of the marker (`crates/nvs-stdlib/src/script.rs:1009` names the interception),
      so this is the unwind asserted rather than built. `rule:errors/propagation` and
      `rule:observability/three-endings-fire-the-exit-queue`.
- [ ] **Write the three cases of stage 3's second check** — a `catch (Throwable)` arm does not admit a
      finish, a finish inside a `try` releases every local of that frame, and a finish that has passed a
      `catch` region is still a finish — at `crates/nvs-codegen/tests/throwing.rs:326`, beside the leak
      cases above. The mechanism is that the marker is a root of its own
      (`crates/nvs-hir/src/errors.rs:83`), so what is asserted is the arm falling through with the
      frame's locals released. `rule:errors/propagation`.

## Backlog

- Stage 4's three remaining `.nvst` cases — `docs/agent/loop-goal.toml:8643` names them.
- Stage 4's `examples/finish.nvs` exact check — `docs/agent/loop-goal.toml:8655`.
- Stage 4's `-p nvs-host` check was not re-run this session — `docs/agent/loop-goal.toml:8620`.
- Stage 5: the goal's record and the rule fragment for the fourth ending and the refusal —
  `docs/agent/loop-goal.md` § *Stage 5*.
- `[context] modules` names no `crates/nvs-types/**` pattern, so the map printed nothing for the crate
  this session edited; the driver sweeps that field from the commits below.
