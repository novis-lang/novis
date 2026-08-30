# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open**: fourteen of its seventeen findings are
ticked, D12 last. The item-34 `[[check]]`'s thirteenth case now exists and passes; its first
unwritten case is the **fourteenth**, `tests/conformance/core/task-after-response-runs.nvst`
(finding M1) — one acceptance failure in the check's order.

**`Core\Script::args()` exists, and answers `mixed`.** `nvs_stdlib::script::CLASS` is a registered
class beside `Core\Script\Handle` — the way `Core\Time` sits beside `Core\Time\Instant` — and its one
row reads `Ctx::isolate_argument` and retains. That module's doc is the one home of the two design
calls: the type is `mixed` because `spawn script`'s `args:` narrows nothing at the call site, and a
child spawned without the option reads `null` rather than an empty array because a parent that wrote
`args: []` said something it did not. **ADR 0012 § 6's body states both now** — it had said
`array<mixed>` and "empty", which the checker and `nvs-ir`'s lowering had each already contradicted.
The new refcount edge is valgrind-clean: `wsl.exe -- bash /mnt/<drive>/<repo>/tools/leak-check.sh
.agent-tmp/leak-args.nvs`, three spawns, two reads each.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1072,
differential 206.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247` and `E07xx` is `E0794`; this session added no diagnostic.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs; **0012 § 6** is this session's proven one, and **0071 §§ 1-2, 5, 7**, **0013 §§ 2-4**,
**0046 §§ 2, 4-5**, **0053 §§ 1-3**, **0007 §§ 2-3**, **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**,
**0047 § 2**, **0011**, **0086 § 6** and **0096 §§ 1-1a** are the earlier ones; for the next group add
**0072 §§ 6-7**. `[context] modules` misses `crates/nvs-runtime/src/ctx.rs`,
`crates/nvs-runtime/src/script.rs` and `crates/nvs-stdlib/src/script.rs`, which this session read, on
top of `crates/nvs-types/src/layout.rs`, `crates/nvs-codegen/src/lib.rs`,
`crates/nvs-ir/src/lower/mod.rs`, `crates/nvs-syntax/src/ast.rs`, `crates/nvs-stdlib/src/json.rs`,
`crates/nvs-types/src/expr/args.rs`, `crates/nvs-types/src/retrieval.rs`, `derive.rs`,
`attributes.rs`, `crates/nvs-stdlib/src/arr.rs`, `serialize.rs`, `crates/nvs-hir/src/errors.rs` and
`crates/nvs-ir/src/lower/call.rs`. `orient.py` itself still warns that
`crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**The item-34 `[[check]]`'s fourteenth case is M1 — `Core\Task::afterResponse` is in spec § 19 and
never landed — and its file set is `crates/nvs-stdlib/src/task.rs` with `[deferred]` in
`crates/nvs-config/src/tree.rs`.** The third item below is D7's remainder and shares neither; take it
only if the first two land early.

- [ ] **M1 `Core\Task::afterResponse` does not exist** — `docs/reference/findings.md:28` lists it under
      item 34, and ADR 0072 § 6 is the rule
      (`docs/adr/0072-core-task-structured-concurrency.md:178`), with § 7's `[deferred]` bound at
      `docs/adr/0072-core-task-structured-concurrency.md:210`. The five edits of a `Core` member on
      `Core\Task`, whose roster is `crates/nvs-stdlib/src/task.rs:108` and whose shared options bag is
      `crates/nvs-stdlib/src/task.rs:100`. The design call: what `nvs run` — a CLI invocation with no
      response for anything to be *after* — does with one, and whether that is the same answer a
      spawned isolate gets.
- [ ] **The case M1 is closed by** — `tests/conformance/core/task-after-response-runs.nvst`, which the
      item-34 `[[check]]` names as its fourteenth. `crates/nvs-stdlib/src/task.rs:108` is the roster it
      has to reach, and the coverage floor of three applies: read this session's new playbook bullet
      before writing a multi-file case, because the floor cannot see an auxiliary `--FILE--`.
- [ ] **D7's remaining halves — an array or enum field is still a `FATAL`** — ADR 0071 § 1.
      `crates/nvs-types/src/derive.rs:240`'s `codec_ty` erases every type it cannot spell to
      `CodecTy::Opaque`, and `crates/nvs-stdlib/src/json.rs:1252`'s `decode_field` is where that
      becomes the fatal a program sees; `crates/nvs-stdlib/src/json.rs:1350`'s `decode_nested` is the
      shape a new arm follows.

## Backlog

- Item 34's other open findings — D9, D10, D16, D17, D21, D22, D1, D33 — `docs/reference/findings.md`.
- Stage 9's items 21–23, ADR 0119's expression `catch` — `docs/agent/loop-goal.md`.
- The website's ADR mirror is a generated copy behind `npm run sync:adrs`, and this session's ADR 0012
  edit is not in it — `website/package.json`.
- `json.rs`'s gap 2: a `decimal`, an `Instant` and an inline shape are still `CodecTy::Opaque` —
  `crates/nvs-stdlib/src/json.rs`.
- The superglobal hint at `crates/nvs-syntax/src/parser/mod.rs:426` still says "that class is still to
  be implemented", now false for `$_ARGS` alone — `docs/adr/0012-no-superglobals.md` § 6.
- `orient.py` warns `crates/nvs-host/src/budget.rs` matches no module — `docs/agent/loop-goal.toml`.
