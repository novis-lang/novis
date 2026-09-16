# Handoff

## State

**Goal `unowned-closures`, stage 6 (the register).** `python tools/owners.py` reads 80 items with
`unowned: 9`, `untagged: 0`, `broken-tag: 0`, `unreasoned: 0`, and `--deferrals` green. The nine left are
in `nvs-host` (three), `nvs-db`, `nvs-runtime` and `nvs-stdlib` (two each).

**`nvs-cli`'s whole share of the register is settled.** The generated API document's gaps 1–3 — a class
response body, a request body, `components.securitySchemes` — are M10's, with `docs/plan/m10.md` now
stating that scope and naming the file; `info.version` is M15's, because a version is what a
`package.toml` publishes and not a deployment key the emitter would quote back; and the enum-case gap is
**struck**, since `RouteParam::admits` has been handing the spellings to `schema` since goal
`m7-server-surface` under a guard test. The test runner's serial suite is M10's too, where `--coverage`
and `--mutate` turn one suite run into many. Both `docs/agent/carried-gaps.md` § *Unowned* bullets for
this crate are gone with them.

**`docs/agent/loop-goal.toml`'s `[context.stage.6]` was thin** — it printed `M10:lead` alone, so a
deferral's plan file and ADR 0184's two sections cost a call each. It now names `M15:lead`, `M9:lead` and
`0184 §2`/`§5`.

## Next group

**Stage 6: the register** — one file set: `crates/nvs-host/src/`. Both items are the same seam, `on:
"worker"`'s destination, and each is a scheduling decision written into the gap's own `— owner:` line: a
goal slug on the chain, or an M9+ deferral whose plan states the scope (`python tools/owners.py
--deferrals` is the gate on the second kind).

- [ ] **A path entry is not placed on another core** — `crates/nvs-host/src/placed.rs:33` gap 1, which is
      that gap's one home, and `crates/nvs-host/src/group.rs:90` gap 1, which states it a second time and
      moves with it. `rule:concurrency/on-worker-runs-the-child-on-another-core`, reasoning in ADR 0184
      § 2. Closing it is a resolver a worker core can reach, so the question is which seam owns the unit
      cache — not the crossing, which is built.
- [ ] **A serving core registers no inbox** — `crates/nvs-host/src/worker.rs:98` gap 1. ADR 0184 § 5
      decides the destination set and its *Revisiting* names the fallback the code took; the question is
      `nvs serve`'s boot order, and `docs/agent/carried-gaps.md` § *Unowned* holds its reason.

## Backlog

- `crates/nvs-db/src/span.rs:65` and `crates/nvs-db/src/tds/mod.rs:88` — the `nvs-db` pair, one file set.
- `crates/nvs-runtime/src/ctx/mod.rs:65` and `crates/nvs-runtime/src/graph.rs:74`.
- `crates/nvs-stdlib/src/cache.rs:155` and `crates/nvs-stdlib/src/response.rs:199`.
- `past-milestone: 8` — items deferred to a milestone behind the program, which goal `gap-zero` makes
  fatal and this goal has not touched.
