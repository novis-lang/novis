---
milestone: post-parity
---
# Loop goal 52 — every plan file and module doc says what the tree does

A reader of `docs/plan/`, the plan's status block or a module doc's `# Known gaps` learns what the tree
does today. Nothing built is described as owed, no owed work is filed under a goal that has already
walked, and every row of the gap index points at a gap that is still open — so the goals after this
one derive what is owed from documents that are true, rather than rediscovering it one stale sentence at
a time.

## Why here

**First of the gap program**, directly after goal `websocket-client`, because every goal behind it reads
these documents to decide what to build. On 2026-09-13 an audit of the past milestones (M0–M8) found more
stale sentences than open items: built work described as owed, and owed work filed under goals that
walked without doing it. A closure goal that starts from those sentences spends sessions rebuilding
what exists or re-proving what is closed. Goal `gap-register` comes next and derives the register from
these files, so they are made true first. Nothing here changes code behaviour — it is text, and the
one tool it adds only reads text.

## Stage 0 — the catch-up

None. This goal *is* a catch-up.

## Stage 1 — the floor

Goal `websocket-client`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: a sentence that defers to a walked goal is found by a tool

`python tools/plan.py --stale` (`tools/plan.py:@main`, beside `--check`). It walks `docs/plan/m*.md` and
the plan's status block and prints every sentence that names a goal the chain has already walked —
`tools/plan.py:@live_goal` and `:@chain_goals` already say which those are — together with a
future-tense marker: *will*, *waits*, *until*, *arrives*, *scheduled*, *still owed*, *not yet*, *the one
that*, *finishes it*. The last line is `sentences deferring to a walked goal: N` — a count written after its
label, because an acceptance `want` is a substring match and `0 sentences` would also match `10
sentences`. It is a lint, not a proof, and its doc says so: it finds the shape that went stale most
often in the audit, and the stages below still read each file. It exits 0 either way; the acceptance
check reads its count.

## Stage 3 — the plan files

One file set: `docs/plan/m*.md` and `docs/implementation-plan.md`. Each sentence is rewritten whole
(AGENTS.md rule 6) to say what is true now, with the `file:line` that shows it in the commit message.
Found by the audit, re-checked before editing:

- **M1** — `docs/plan/m1.md:3-5` and `:92-97` still call the pipeline operator owed and name goal
  `surface` as its carrier; it is built (`crates/nvs-syntax/src/token.rs:271` `PipeGreater`,
  `crates/nvs-syntax/src/lexer.rs:1021`, `E0129`–`E0131`, `tests/conformance/reject/pipeline/`).
  `:38-41` says one-equality-operator is not built; `crates/nvs-syntax/src/lexer.rs:1039`/`:1048` refuse
  `===`/`!==`. `:102-103` schedules the PHP 8.6 refusals later; `crates/nvs-syntax/src/token.rs:487`/
  `:491` and `tests/conformance/reject/php86/` hold them. `:72-74`'s literal-types sentence
  predates `rule:types/literal-types`' checked conversion (`E0469`).
- **M2** — `docs/plan/m2.md:40` says no equality-operand check exists; `crates/nvs-types/src/locals.rs:1230`
  calls `reject_disjoint_equality`. `:50-51` "the transform itself may land later" defers the generator
  state machine to no owner — check whether it landed and say so.
- **M3/M4** — `docs/plan/m3.md:21-22` and `docs/plan/m4.md:8-9` say `backtrace` is string form only;
  `crates/nvs-types/src/error_lib.rs:334` asserts `array<string>`. `docs/plan/m4.md:3-6` calls the
  1000-case corpus figure the one thing left; `tests/conformance` holds far more.
- **M4S** — `docs/plan/m4s.md:3-5` names goal `core-depth` as the goal that finishes it; the spec ratchet
  `crates/nvs-stdlib/tests/spec-members-outstanding.txt` holds no keys.
- **M4B** — the trivia-in-goal-`doc-comments` paragraph is met.
- **M5** — `docs/plan/m5.md`'s *Verify* says a `callable` variable in `Task::all` is a compile error;
  the rule says the opposite and `crates/nvs-types/tests/core_members.rs:401-407` pins it. `:43-44` is met.
  `:44-47`'s spawn trace event is still owed — that sentence names goal `m5-proofs` instead of "once the
  bits exist". `:22`'s `race` is "deferred with a named future spelling" and names no owner: name one.
- **M6** — `docs/plan/m6.md:5` says the control socket waits for goal `server`; `:42` and `:50` say the
  reload client and `nvs ctl config` arrive with M7. Both name goal `m7-server-surface` instead.
- **M7** — `docs/plan/m7.md`'s first paragraph names the fan-out's carrier twice and inconsistently, and
  its carrier list omits three goals; `nvs serve` runs one worker per core
  (`crates/nvs-cli/src/serve.rs:427-455`). The "Not yet named here" raw-body sentence is goal
  `m7-server-surface`'s to settle; this goal only stops it pointing at "whoever designs" it.
- **M8** — `docs/plan/m8.md`'s first paragraph says goal `test-request` lands `Core\Test::request` whole;
  its second says the classes ratchet is empty once goal `xml-tree` is green, and
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:16` still holds `Core\Metrics`.
- **The status block and table** — `docs/implementation-plan.md:19-26` (M1's pipeline owed, M4's corpus
  figure, M4S's two `signed-urls` keys), `:104-105`, `:111-114` (goals named by number). `M4`'s `done*`
  cell is regenerated by `python tools/plan.py --sync` now that goal `m4-refusals` carries it.

## Stage 4 — the module docs

One file set: the crates' `//!` docs. A stale gap is **struck with its evidence**, never re-pointed; a
module doc is rewritten whole where it changes.

- `crates/nvs-cli/src/serve.rs:79-90` — "a served request carries no configuration" (tagged M6) is closed:
  `crates/nvs-server/src/serve.rs:1101` hands the snapshot to every request and `:5928` pins it.
- `crates/nvs-cli/src/serve.rs`'s one-core wording, wherever it survives — closed by `:427-455`.
- `crates/nvs-syntax/src/lib.rs:57-64` still says `|>` is not a token and must not become one.
- `crates/nvs-runtime/tests/perf_guards.rs:1118` says `PropertyObserver` has no implementation;
  `crates/nvs-stdlib/src/interfaces.rs:48` registers it and `crates/nvs-runtime/src/object.rs:3682`
  dispatches it.
- `crates/nvs-test/src/lib.rs:150-151` says `nvs.toml` is not read until M6; M6 has walked. Say what
  `--INI--` does now, or record why it stays refused.
- **Module-doc gaps the code already closes**, found by the classification of every `unowned` item on
  2026-09-13. Each is struck with the evidence beside it, re-checked first:
  - `crates/nvs-ir/src/lib.rs` gap 3 (tagged arithmetic — `crates/nvs-ir/src/lower/operator.rs:760-800`),
    gap 4 (integer into enum — `crates/nvs-ir/src/lower/convert.rs:1490`, `:1655`), gap 5 (mismatched arms
    join at `Ty::Tagged` — `crates/nvs-ir/src/lower/expr.rs:1718-1727`), gap 6 (static property store —
    `crates/nvs-ir/src/lower/stmt.rs:1174-1186`), gap 9 (closure argument tags —
    `crates/nvs-runtime/src/closure.rs:36-47`), gap 15 (`<=>` rows — `operator.rs:69`, `:736`, `:964`),
    gap 16 (`**` — `operator.rs:1028`), gap 17 (one flat map is the scoping rule —
    `crates/nvs-ir/src/lower/mod.rs:1446-1449`), gap 19 (its own body records every row as lowering),
    gap 20 (literal membership miss edge — `crates/nvs-ir/src/lower/convert.rs:1636-1675`).
  - `crates/nvs-runtime/src/ctx/mod.rs` gap 1 (the stack ceiling is discovered —
    `crates/nvs-host/src/scheduler.rs:780-784`).
  - `crates/nvs-types/src/lib.rs` gaps 1–3 (disjoint equality — `crates/nvs-types/src/expr/operators.rs:416`;
    every path returns — `crates/nvs-types/src/check.rs:817-842`; `is` narrows —
    `crates/nvs-types/src/locals.rs:557-593`, owner goal `type-test` retired);
    `crates/nvs-types/src/locals.rs` gap 1 (the fall-through answer is exact, so it is no gap);
    `crates/nvs-types/src/error_lib.rs` gap 1 (`?->` reads through —
    `tests/conformance/error/a-throwable-is-constructed-by-name.nvst:23`);
    `crates/nvs-types/src/intrinsics.rs` gap 6 (`nvs check` reads the grants —
    `crates/nvs-cli/src/main.rs:1288-1291`); `crates/nvs-types/src/signatures.rs` gaps 1–2 (promoted
    properties — `:1117-1150`; a variadic is bound as `array<T>` — `crates/nvs-types/src/check.rs:655-657`).
  - `crates/nvs-stdlib/src/lib.rs` gap 4 (`array<T>` is covariant — `crates/nvs-types/src/expr/assign.rs:159-175`,
    owner goal `unowned-sweep` retired) and `crates/nvs-stdlib/src/response.rs` gap 2 (`E0801` refuses two
    typed writers — `crates/nvs-types/src/response.rs:48-55`).
- **`rule:types/arrays`' *Invariant.* bullet is behind the code.** `crates/nvs-types/src/expr/assign.rs:159-175`
  makes `array<T>` covariant in its element and argues why that is sound for a copy-on-write value. Under
  the user's rule the code wins: the fragment's bullet (and its `diverges:` line, and
  `docs/rules/types/class-reference-variance.md`'s contrast sentence) is amended to the code by the rule's
  own process, `crates/nvs-stdlib/src/lib.rs` gap 4 is struck, and — if no test pins the covariance yet — a
  conformance case does (`Core\Arr::flip` over an `array<string>`). No record is opened; the frozen one
  stays history.
- **The six items `python tools/owners.py` lists under *owners that went green without closing the gap*.**
  Each is judged against the code: closed → struck with evidence; open → its owner becomes the goal
  that now carries its file set (`m7-server-surface`, `m8-db-queue`, `m8-stdlib-depth`) or `unowned`,
  for goal `unowned-closures`.

## Stage 5 — the index

`docs/agent/carried-gaps.md`, which goal `gap-zero` deletes at the end of the program. Until then it
must not lie. `python tools/playbook.py --check` lists the § *Owned* rows whose owner walked; each is
closed (struck, with the `file:line` that closed it) or its owner is struck for the goal that now
carries it. That report prints nothing at all when no row is flagged
(`tools/playbook.py:@report_expiry`), which no acceptance check can read; it gains a
`none -- every carried-gaps owner is live or struck` line, the shape its neighbours already print.
Known closed at authoring time: the `array<T>` cycle
(`crates/nvs-runtime/src/object.rs:4479`), `Core\Request::clientIp`/`host`/`scheme`, one-core serving,
`Core\Net`/`Os`/`Signal`, the roster of ~110 untagged items, `Core\Uri::with`. Re-check each.

## Standing decisions

- **The user's rule for the whole gap program, settled 2026-09-13**: every gap is closed or deferred to
  a future milestone, **M9 and above**. M0–M8 — M4S and M4B included — are complete when goal `gap-zero`
  goes green, and every promise their plan files made is built. A deferral names M9+ only when the item
  cannot be built without that milestone's work, never because it is large.
- **This goal writes no code** except `plan.py --stale`, and changes no behaviour. A sentence whose truth
  needs a code change is not rewritten to hide it: it names the goal that owns the change.
- **Ahead of the document, the code wins; behind it, the document names a gap.** The user's rule,
  2026-09-13. Where the tree does *more than* or *other than* a plan sentence, a rule or an earlier
  decision says, and it is implemented, tested and verified, the sentence is rewritten to the tree and a
  frozen record stays as history. Where the tree *lacks* something the document promised, the sentence
  is not softened: it names the goal that builds it, and the gap is recorded in the owning module doc.
  If a session cannot tell which of the two it is facing, that is a `BLOCKED` for the user. Either way a
  claim with no `file:line` behind it is not made (AGENTS.md rule 7).
- **Frozen records stay frozen.** A decision record under `docs/decisions/` is history and is never
  edited here; a rule fragment is edited only where it states something the tree contradicts, and then
  by the rule's own process.
- **What it spends**: nothing at run time. One read-only tool mode.
- **ADR slots**: none.
- **Not this goal**: building anything the stale sentences describe as owed — each is named to the
  closure goal that builds it; the register tooling (goal `gap-register`); deleting the index (goal
  `gap-zero`).
