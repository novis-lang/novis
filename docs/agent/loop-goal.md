---
milestone: post-parity
---
# Loop goal 60 — every unowned gap is built to the answer its decision sheet gave

No module-doc gap is `unowned` any more. Each one is **built**, **struck** because the user's answer made
it a stated bound of the design rather than a hole, or **tagged to a future milestone** whose plan states
its scope. The design calls that were holding them were taken by the user in one sitting on a decision
sheet, and each answer is written in the gap's own module doc as a `Decided:` sentence before this goal
starts, so no session re-decides one.

## Why here

Last of the closure goals, directly in front of goal `gap-zero`, because it is the only one that needs the
user: goals `m4-refusals` through `m8-stdlib-depth` build what the past milestones' plans promised and
need no answer from anyone, while roughly sixty of the gaps here waited on a design choice nobody had
taken. The user chose on 2026-09-13 to answer them on a sheet rather than let the loop hold on each one.
Goal `gap-zero` after it retires `unowned` as an owner kind, which is only safe once this goal has
emptied it.

## Stage 0 — the catch-up: every decision is written where its gap lives

**Done by hand before the first session**, from the user's decision sheet: every gap it covered carries
a sentence `Decided: <the option> — <why, in one line>.` as the last prose line before its owner tag,
and the tag names this goal. A gap a session reaches with **no** `Decided:` sentence and no obvious
build is a `BLOCKED` naming the gap — the one hold this goal expects, and it means the sheet missed it.

## Stage 1 — the floor

Goal `m8-stdlib-depth`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the lowering and the runtime

One file set: `crates/nvs-ir/src/lower/`, `crates/nvs-runtime/src/`, `crates/nvs-codegen/`.

- **Security first.** `crates/nvs-ir/src/lib.rs` gap 11 — a `secret` compared against a `mixed` falls to
  the short-circuiting `Identical` (`crates/nvs-ir/src/lower/operator.rs:896-910`); when the checker
  recorded `ExprInfo::SecretEquality` and one side is `Ty::Tagged`, a helper untags a string or bytes
  operand and compares in constant time, answering `false` for any other tag.
  `crates/nvs-runtime/src/budget.rs:89` gap 1 (one allocation past the budget) to its decision.
- **Builds with no choice left**: `crates/nvs-ir/src/lib.rs` gap 1 (a multi-condition `for`, and a label
  at a foreign representation compared through `lower_binary` — `crates/nvs-ir/src/lower/control.rs:460-465`,
  `:680-683`, `crates/nvs-ir/src/lower/expr.rs:1825-1827`) and gap 2 (the normalized subscript key on
  `owned_temporaries`, `crates/nvs-ir/src/lower/mod.rs:2091-2101`) — **unless goal `m4-refusals` already
  closed them as refusal sites; check first**; gap 21 (a hook marker on the descriptor's property row, read
  by `nvs_object_key_get` and the erased write — `crates/nvs-runtime/src/object.rs:3403-3408`).
- **Decided**: `crates/nvs-runtime/src/lib.rs` gaps 1, 2, 6, 7 (value tags, string reuse, runtime
  exception context, the in-flight collector); `crates/nvs-ir/src/lib.rs` gap 18 (a throw from an
  abandoned generator's `finally`); `crates/nvs-runtime/src/array.rs` gap 1, `commands.rs` gap 1,
  `decimal.rs` gap 1, `routes.rs` gap 1.

## Stage 3 — the checker and the front end

One file set: `crates/nvs-types/src/`, `crates/nvs-hir/src/`, `crates/nvs-syntax/src/`,
`crates/nvs-diagnostics/src/`.

- **Builds**: `crates/nvs-hir/src/requires.rs` gap 2 (the harvest's wildcard arms made exhaustive,
  `requires.rs:703`, `:705`, `:910`, `:975`, `:1018`, `:1191`, `:1245`), gap 3 (`Probe::tried` folded into the
  unit key, `crates/nvs-config/src/cache.rs:36-37`, `crates/nvs-cli/src/script.rs:164`);
  `crates/nvs-types/src/ctor_init.rs` gap 2 and `lateinit.rs` gap 2 (both `scan_expr` walks exhaustive,
  closure bodies deliberately not counted).
- **M1's two, which no goal on the chain had taken** (tagged `M1`, a milestone that must be complete):
  `crates/nvs-syntax/src/lib.rs` gap 2 — `use function` / `use const` get the targeted refusal the gap
  itself asks for, naming `rule:classes/no-free-functions-or-constants`, instead of a generic parse error;
  gap 3 — keyword-spelled name segments past the first are covered by conformance cases rather than
  spot-checked (`Parser::is_name_segment`). Both are retagged to this goal in stage 0.
- **Decided**: `ctor_init.rs` gaps 1, 3; `lateinit.rs` gaps 1, 3; `defaults.rs` gap 1; `derive.rs` gap 3;
  `intrinsics.rs` gaps 1–5; `links.rs` gap 1 and `reasons.rs` gap 1 (with `crates/nvs-stdlib/src/router.rs`
  gap 1, the same question); `response.rs` gap 1; `crates/nvs-syntax/src/casing.rs` gap 1,
  `crates/nvs-syntax/src/lib.rs` gap 1; `crates/nvs-hir/src/hierarchy.rs` gap 1, `requires.rs` gap 1;
  `crates/nvs-diagnostics/src/embedded.rs` gap 1.

## Stage 4 — the library

One file set: `crates/nvs-stdlib/src/`.

- **Builds**: `compress.rs` gap 1 (`Core\Compress\Stream`, shaped like `crates/nvs-stdlib/src/hash.rs:434`,
  one `Bound` per inflating stream); `task.rs` gap 1 (the result shape's per-slot representations recorded
  at the call site, `crates/nvs-ir/src/lower/mod.rs:3352-3363`).
- **M6's two, which no goal on the chain had taken** (tagged `M6`): `regex.rs` gap 2 — the step budget
  becomes a `[limits]` directive with today's constant as its default; gap 3 — the per-core
  compiled-pattern cache is charged to an accounting bracket, or replaced by the prepared-pattern
  channel if stage 3 builds it for regex literals too. Both are retagged to this goal in stage 0.
- **Decided**: `cli.rs` gap 1; `command.rs` gap 1; `db/mod.rs` gaps 1–2; `debug.rs` gaps 1–2; `json.rs`
  gaps 1–5; `lib.rs` gap 1; `mime.rs` gap 1; `path.rs` gaps 1–2; `queue.rs` gaps 2–3; `random.rs` gap 1;
  `regex.rs` gap 1; `test.rs` gaps 1–2; `uuid.rs` gap 1; `xml.rs` gap 1; `zip.rs` gaps 1–2;
  `cldr.rs` gap 1 and `time.rs` gap 1 (the prepared-pattern channel, the same answer as `intrinsics.rs`
  gap 2 in stage 3 — build it once).

## Stage 5 — the server, the configuration, the cache and the schema

One file set: `crates/nvs-server/src/`, `crates/nvs-config/src/`, `crates/nvs-cli/src/cache.rs`,
`crates/nvs-db/src/`.

- **Builds**: `crates/nvs-config/src/cache.rs` gap 1 (a `build.rs` stamping a hash of the compiler's
  source into `compiler_version_hash`); `crates/nvs-server/src/bounds.rs` gap 1 (`[server]` keys for the
  idle, lifetime, frame-size and open-connection bounds, beside `nvs_config::server::waits_for`).
- **M6's `env_hash`** — `crates/nvs-cli/src/cache.rs` gap 1 (tagged `M6`): the unit key's environment
  hash tells two builds of an unreleased tree apart, with the same source stamp as `nvs-config`'s build
  hash above. Retagged to this goal in stage 0.
- **Decided**: `crates/nvs-server/src/route.rs` gap 1 (where a forged CSRF token is refused);
  `crates/nvs-cli/src/cache.rs` gaps 2–3 (`aarch64` and Mach-O); `crates/nvs-db/src/catalog.rs` gaps 1–3,
  `ddl.rs` gaps 1–2, `schema.rs` gap 1.
- **Goal `m7-server-surface`'s three open items**, re-owned here because that goal retired without
  closing them: `crates/nvs-server/src/metrics.rs` gap 1 (the `otlp` pusher behind `[metrics] endpoint`,
  its stage 10); `crates/nvs-server/src/schedule.rs` gap 1 (a fire's context carries the deployment's
  configuration, so its `limits` sub-cap has a ceiling to narrow and `script` is granted, its stage 9);
  `crates/nvs-server/src/trace.rs` gap 1 (the gate that files a `query`, an `http` and a `spawn` event
  for a sampled request without `DebugFlags::TRACE`, its stage 11). The same goal's last
  `docs/agent/carried-gaps.md` row comes here with them: spec § 13's `Core\Test` cell spells `request`'s
  bag nowhere, and `crates/nvs-stdlib/src/test.rs`'s `REQUEST_OPTIONS` is the roster it should state.

## Stage 6 — the honest deferrals

Five items genuinely cannot be built before M10's debugger and reference index exist, and are **retagged
`M10`**, with `docs/plan/m10.md` gaining the scope sentence where it does not already carry one:
`crates/nvs-ir/src/lib.rs` gap 14 and `crates/nvs-runtime/src/lib.rs` gap 5's `DEBUG_BREAK` half (`nvs
dap`, `docs/plan/m10.md:19`); `crates/nvs-lsp/src/completion.rs:191`'s items; `crates/nvs-lsp/src/hints.rs`
gap 1; `crates/nvs-lsp/src/index.rs` gaps 1–4. The items already tagged to a future milestone are held
to the same test: the six `crates/nvs-fmt/src/lib.rs` gaps (M10), `crates/nvs-ir/src/lib.rs` gap 7 and
`crates/nvs-runtime/src/decimal.rs` gap 2 (M12), and `crates/nvs-cli/src/bundle.rs` gap 1 (M9) — each
milestone's own file states the scope, or gains the sentence here. `python tools/owners.py --deferrals`
is the proof.

## Standing decisions

- **The user's rules, settled 2026-09-13**: every gap is closed or deferred to M9+, and a deferral is
  honest only when the item cannot be built without that milestone's work. **Code ahead of a decision
  wins; code behind one is a gap.** Where implemented, tested and verified code goes *beyond* or
  *differs from* a decision record or an earlier decision, the code counts: the rule fragment, plan or
  module doc is rewritten to match and the record stays frozen. Where the code *lacks* something a
  decision specifies, that is a gap to build — never a reason to rewrite the decision down to what
  exists. Where it is unclear which of the two it is, that is a `BLOCKED` for the user.
- **A `Decided:` sentence is not re-opened.** A session that finds the chosen option harder than the
  sheet priced builds it anyway, or records the obstacle in the handoff and takes the next item — never
  the other option silently.
- **An answer of "state it as a bound" strikes the gap**: the bound is written as the module's own prose
  (what it does, and the limit), with the rule fragment amended if the rule promised otherwise. A bound is
  not a gap.
- **A decision whose answer changes a rule** amends that rule's fragment by its own process in the same
  slice, and opens no record: the sheet is the decision, and a rule's `because` may cite the gap it closed.
- **Three answers differ from the sheet's recommendation, deliberately.** A runtime-raised exception
  captures a full backtrace (`crates/nvs-runtime/src/lib.rs` gap 6), which amends
  `rule:errors/throw-is-not-slower` for runtime-raised exceptions and states what a raise now costs;
  `Core\Queue\Stats` gains a fifth counter (`crates/nvs-stdlib/src/queue.rs` gap 3, a spec § 6
  amendment); `Core\Random\Seeded` is registered per spec § 11 (`crates/nvs-stdlib/src/random.rs` gap 1).
  Each is built as decided and none is re-opened.
- **ADR slots**: one new record, and no other number — for the prepared-pattern channel from the checker
  to the lowering, if the user's answers to `intrinsics.rs` gap 2 and `cldr.rs`/`time.rs` gap 1 build it.
- **What it spends** is decided item by item and written in each module doc per
  `rule:programs/memory-priority`; the sheet's options already priced it.
- **Not this goal**: anything a past milestone's plan promised (goals `m4-refusals` through
  `m8-stdlib-depth`); the terminal gate (goal `gap-zero`).
