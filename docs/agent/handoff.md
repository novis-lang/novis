# Handoff

## State

**Goal `m5-proofs` (M5). Stage 6's first item is on disk: all five `spawn script` options are checked
where they are written, and the placement refusal has a code.**

`limits:` is a shape of the `[limits]` sub-caps a program may narrow, each optional, with a key that is
not one of them refused as `E0454` rather than accepted by width subtyping; `grants:` is an
`array<string>` of capability names in `nvs.toml`'s spelling; `on:` is one of two written words,
`E0818` otherwise. `crates/nvs-types/src/expr/isolate.rs:120`'s doc is the home of which type and why,
and the key set itself is `SUB_CAP_SETTINGS`/`SUB_CAP_COUNTS` in that file.

`E0777` is retired the way `E0776` is — the option it refused is now checked — and the five sites whose
prose said so are rewritten: the lowering's doc, `nvs-ir`'s refused-in-a-body table, `Core\Socket`'s and
`Core\Queue`'s module docs, and the two `.nvst` cases.

**Checked is not applied.** Nothing below `nvs-types` carries any of the three yet: lowering drops them
(`crates/nvs-ir/src/lower/expr.rs:3122`), so a `limits:` narrows nothing, a `grants:` narrows nothing and
every child starts on this core. The next two items are what closes that, and until they land a program
that writes one of the three gets the compile-time check and no run-time effect.

## Next group

**Stage 6: the options, below the checker** — one file set: `crates/nvs-ir/src/lower/expr.rs`,
`crates/nvs-stdlib/src/script.rs`, `crates/nvs-host/src/group.rs`, `crates/nvs-host/src/isolate.rs`, a
new module beside them, and `tests/conformance/isolate/`.

- [ ] **The placement reaches the host seam** — lowering passes `on:` as one more argument beside the
      path, `args:` and `output:` at `crates/nvs-ir/src/lower/expr.rs:3188`, the symbol's row takes it at
      `crates/nvs-stdlib/src/script.rs:619`, and the seam routes it at
      `crates/nvs-host/src/group.rs:190`. The word crossing is `"here"`/`"worker"` as written;
      `rule:concurrency/on-worker-runs-the-child-on-another-core` ¶ 1 is what a placement means, and
      `crates/nvs-ir/src/lower/expr.rs:3122`'s doc is the sentence this item makes false.
- [ ] **The worker cores and their inbox** — a new module under `crates/nvs-host/src/`, built on the
      handoff shape at `crates/nvs-host/src/blocking.rs:14-20` turned around and the cross-thread wake at
      `crates/nvs-host/src/reactor.rs:219`, bounded at the core count as
      `crates/nvs-host/src/blocking.rs:29-36` bounds its pool. The child is *started* there and never
      migrated: `rule:concurrency/on-worker-runs-the-child-on-another-core` ¶ 3-4 and
      `rule:concurrency/a-wake-never-moves-a-task`.
- [ ] **The three `.nvst` cases** `docs/agent/loop-goal.toml:9850` names, under
      `tests/conformance/isolate/` — a worker-placed child answering as a local one does, a child that
      holds only its grants, and one stopped at its own ceiling.
      `rule:security/isolate-budget-is-the-trees` and `rule:security/isolate-shares-nothing` are what the
      last two assert; the enforcement they need is the item above plus the budget seam.

## Backlog

- `limits:`/`grants:` enforcement lands in this stage; when it does, `Core\Queue::push` can declare both
  — `crates/nvs-stdlib/src/queue.rs`'s gap 1 is the home of that condition.
- `tests/conformance/core/a-socket-upgrade-declines-the-options-its-sibling-does-not-enforce.nvst`'s
  *name* goes stale once the sibling enforces them; rename it in the item that does.
- `rule:observability/spawn-is-its-own-event` says the child's wall time "already arrives on
  `ScriptResult`", and that shape's fields (`crates/nvs-stdlib/src/script.rs:428`) do not carry it —
  the overhead split reads it off the native `Completion` instead. Either the rule's sentence or the shape.
- `[context] adrs = ["0006"]` gives *In short* only; a session needing § *Decision* slices it by hand.
- `Core\Server`'s request-reading members and `traceId()` are `crates/nvs-stdlib/src/server.rs:11-14`'s
  known gap, not this goal's.
- `docs/plan/m5.md`'s stale prose and the module-doc gaps tagged `unowned` — goals `plan-truth`'s and
  `unowned-closures`'s.
