# Handoff

## State

**Stage 6's adversarial suite is green, and with it every `-p nvs-config` check the goal names.**
The three cases the acceptance list names are on disk:
`a_script_attempting_to_set_a_system_directive_fails` and
`a_script_attempting_to_widen_a_capability_fails` in `crates/nvs-config/tests/request.rs`, and
`spawn_script_without_the_capability_fails` in `crates/nvs-config/tests/capability.rs` (new).

All three are **sweeps with a positive control**, not single-key assertions: the first refuses every
`System` row of `DIRECTIVES` in both its spellings and then asserts `request.all()` is byte-identical
to before, so a `set` that returned `false` after writing the overlay fails; the second refuses every
row of `Cap::ALL` against three values and then asks `Capabilities::allows` — not `get`, which has no
rendering for a list — that the grant in force is still the file's. Each ends by accepting a
`Runtime` directive, so a `set` that refused everything cannot pass any of them.

**The `RuntimeTighten` refusal has two reasons and the case holds both.** A capability with a grant
in force is refused because a list cannot be shown to narrow; every other row is refused for having
nothing to narrow *from*. Neither is the value's doing — `log.level` is the same unquantifiable shape
under `Runtime` and is accepted — which is what pins the refusal on the class. `request.rs`'s module
doc owns why refusing is the safe direction here.

`crates/nvs-config/tests/capability.rs` asserts the **rule** with no compiler and no request in front
of it, as `capability.rs`'s module doc asks: a `Capabilities` deserialized from a `[capabilities]`
block, canonicalized once as the snapshot's build does, and a fake `Files` holding a fixed set of
paths plus one symlink. That is what lets `-p nvs-config` own the rule while `-p nvs-stdlib` owns the
diagnostic.

## Next group

**Stage 7 — ADR 0048's bundler, items 19 and 20, and the last two acceptance checks before Stage 8's
counts.** One file set: `crates/nvs-cli/src/main.rs` with a new `crates/nvs-cli/src/bundle.rs`, plus
`tools/try.py`. **Read ADR 0048 first** — the pack will not print it until `[context] adrs` carries
it (see `## Backlog`).

- [ ] **`nvs build --compile` appends the entry file's statically-resolved `require` graph to the
      host binary as plain source** — ADR 0048's *Decision*, which is the only copy of the scope and
      of the source-not-artifacts trade. The `Build` variant takes the flag beside `openapi`, and
      `run_build` is where it branches. Anchors: `crates/nvs-cli/src/main.rs:208`,
      `crates/nvs-cli/src/main.rs:583`.
- [ ] **`python tools/try.py --bundle <FILE> --expect <LINE>`** — the harness the goal owes itself
      (`docs/agent/loop-goal.md` § *The harness this goal owes*), and item 20's "runs identically to
      `nvs run`" is a comparison rather than an assertion about one output. It is a flag on the
      parser that is already there. Anchors: `tools/try.py:178`.
- [ ] **The two `-p nvs-cli` cases the check names** —
      `a_bundle_carries_the_statically_resolved_require_graph_as_source` and
      `a_bundled_executable_runs_identically_to_nvs_run`, over whatever `run_build` ends up calling.
      Take this only after the first two land. Anchors: `crates/nvs-cli/src/main.rs:583`.

## Backlog

- `[context] adrs` should now drop **0074 §§ 2-3** and **0118 § 7** (both landed) and carry **ADR
  0048's `Decision`**, which the next group cannot start without (`docs/agent/loop-goal.toml`).
- `[context] modules` names `crates/nvs-host/src/budget.rs`, which never existed, and names no
  `tools/` file at all — the next group edits `tools/try.py` (`docs/agent/loop-goal.toml`).
- Stage 8's checks want conformance ≥ 1050 and differential ≥ 210 against 1011 and 206 on disk; that
  is the run's last gate and the largest remaining item (`docs/agent/loop-goal.toml`).
- Item 18's fourth clause, "a child cannot widen a capability its parent narrowed", is a conformance
  case in Stage 8's own list rather than a `-p nvs-config` test (`docs/agent/loop-goal.md`).
