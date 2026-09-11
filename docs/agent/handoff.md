# Handoff

## State

**Goal `resource-ceilings` — every resource ceiling stops the request that breaks it. Stage 0 is
landed and nothing else of the goal is.** Goal `editor-surfaces`'s whole acceptance list is still
this goal's floor and is untouched.

The three cases are on disk under `tests/conformance/error/` and **all three are red, which is what
stage 0 is for**: the CPU one runs out `nvs-test`'s 60 s `CASE_TIMEOUT` because nothing samples a
clock against `[limits] cpu_time`, and both memory ones print *nothing at all* — the tier-1 handler
is entered holding more than the widened ceiling and abandoned by `rule:errors/on-limit`'s
zero-retry rule, so not even `onLimit memory` reaches stdout.

**So `python tools/verify.py`'s conformance step and CI's conformance job are red until stages 3, 4
and 5 land**, with three failures and no others, and every run of that tree costs an extra 60 s for
the CPU case. That is the goal's own design (`docs/agent/goals/40-resource-ceilings.md` § *Stage 0*:
*all three run red the day they are written*) rather than a regression to bisect.

Nothing is blocked. Stage 2 is untouched.

## Next group

**Stage 2: the poll word moves out of `Ctx`** — one file set, `crates/nvs-runtime/src/ctx/` plus the
single emit site. The field surface is small and was measured this session: `grep -rn
'self\.safepoint\|ctx\.safepoint\|safepoint:' crates/ --include=*.rs` is **21 lines in five files**,
and every one of the other ~180 `safepoint` mentions in the tree is the IR instruction, not the
field.

- [ ] **Move the word out of line and give the tree one of it** — the field at
      `crates/nvs-runtime/src/ctx/mod.rs:234` becomes a handle to an `AtomicU64`, built where
      `deadline`'s is at `crates/nvs-runtime/src/ctx/wiring.rs:95` and cloned by `Ctx::child` at
      `crates/nvs-runtime/src/ctx/isolate.rs:271`. `Self::deadline`'s field doc at
      `crates/nvs-runtime/src/ctx/mod.rs:264` is the worked example for every decision here, down to
      the *what it spends* paragraph. `rule:security/isolate-shares-nothing` is why a child may not
      be born clean; `SAFEPOINT_OFFSET` at `crates/nvs-runtime/src/ctx/mod.rs:1262` and the
      arrangement `the_hot_words_come_first_and_are_a_word_apart` pins are unchanged, a pointer
      being the same eight bytes.
- [ ] **Read and write through the handle** — the accessors at
      `crates/nvs-runtime/src/ctx/safepoint.rs:41`, `:99` and `:110`, the slow path's three arms at
      `crates/nvs-runtime/src/ctx/safepoint.rs:185`, `:239` and `:258`, and the handler's own
      lowering and raising of `CPU_LIMIT` at `crates/nvs-runtime/src/ctx/hooks.rs:518`. Only
      compiled code follows the raw pointer. `rule:errors/on-limit`.
- [ ] **Hoist the handle into the ABI entry block** —
      `crates/nvs-codegen/src/emit.rs:@emit_safepoint` loads at
      `crates/nvs-codegen/src/emit.rs:1052` today; the handle is bound once beside `ctx_p` at
      `crates/nvs-codegen/src/emit.rs:161` and the back edge holds one load, one test and one
      branch. `rule:testing/perf-two-mechanisms`, and the goal's § *Standing decisions* forbids
      folding the word back into `Ctx` whatever the register pressure turns out to be.

## Backlog

- Stage 3, the CPU sampler in `crates/nvs-host/src/watchdog.rs` — the goal's highest-severity stage,
  and a small addition on top of stage 2 — `docs/agent/goals/40-resource-ceilings.md` § *Stage 3*.
- The three stage-0 cases need no further edit; they go green as stages 3, 4 and 5 land, one each —
  `docs/agent/goals/40-resource-ceilings.md` § *Stage 0*.
- `[limits] cpu_time = "200ms"` in the CPU case is sized for the green day; the 60 s it costs today
  is the case timeout, not the ceiling — `crates/nvs-test/src/run.rs:46`.
- Stage 6's first bracket is `Core\Cache::local`'s `thread_local` — `crates/nvs-stdlib/src/cache.rs`.
