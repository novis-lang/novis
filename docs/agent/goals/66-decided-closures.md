---
milestone: post-parity
---
# Loop goal 66 — every gap the decision sheet answered is built to its answer

No module-doc gap names this goal any more. Each of the forty-eight it starts with is **built** to the
`Decided:` sentence it carries and its numbered item deleted, **struck** because that sentence made it a
stated bound of the design rather than a hole, or **deferred** to a milestone at M9 or later whose own
plan file states the scope — and a deferral is honest only when the item cannot be built without that
milestone's work. `python tools/owners.py --closes decided-closures` is the gate, and it is red while any
item names the goal: a tag is not a build.

## Why here

Directly in front of goal `gap-zero`, in the slot goal `unowned-closures` held. That goal was reached
with forty items tagged to it and none of them built, because its own gate was `owners.py`'s
`unowned: 0` and tagging every item to the goal is what made the count zero; the floor check that names
a goal which walked without closing its gaps reads a goal as retired only after the driver retires it,
so it went red one goal late, under goal `class-scoped-types`. Those forty carry the user's own answers
from the decision sheet of 2026-09-13, written as a `Decided:` sentence in each gap, so nothing here is
re-decided — it is built. Eight more items ride with them: the ones deferred to M1, M6, M7 and M8, a
milestone the program has passed, which `gap-zero` refuses on arrival and no live goal owned. Goal
`gap-zero` builds nothing and cannot absorb any of them; this goal is where they close.

## Stage 0 — the catch-up

None. Every item already names this goal — `grep -rn "owner: decided-closures" crates/` is the whole
list — and forty of them already carry their `Decided:` sentence. The eight without one state what
closes them in their own prose. An item a session reaches with no sentence and no stated build is a
`BLOCKED` naming the gap: the one hold this goal expects.

## Stage 1 — the floor

Goal `cache-shared-dial`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never
traded.

## Stage 2 — the runtime and the lowering

One file set: `crates/nvs-runtime/src/`, `crates/nvs-ir/src/`, `crates/nvs-server/src/metrics.rs`.

- **The in-flight collector**, one build for three items: `crates/nvs-ir/src/lib.rs` gap 14 (`COLLECT`
  is cleared and ignored), `crates/nvs-runtime/src/lib.rs` gap 5 (`nvs_safepoint` acts on nothing) and
  gap 7 (a cycle is reclaimed at teardown). *Decided: a collector that runs only near the memory
  ceiling*, so the normal request pays nothing and a request that would trip its cap gets one pass first.
  State what it spends per request in the runtime's module doc.
- **`crates/nvs-runtime/src/lib.rs` gap 2** — `concat` and `concat_n` reuse a solely-owned left operand,
  so `$s = $s . $x` is linear like append.
- **`crates/nvs-runtime/src/decimal.rs` gap 1** — a division whose 128-bit fold overflows retries at 192
  bits rather than throwing.
- **`crates/nvs-ir/src/lib.rs` gap 18** — a throw escaping an abandoned generator's `finally` is reported
  through the escalation ladder and replaces nothing.
- **`crates/nvs-runtime/src/record.rs`**: gap 1 refuses at compile time a `secret` stored into an array
  element or a shape field (the refusal is `nvs-types`'s; the gap lives where the walk that would meet
  the value does); gap 2 is a bound — through `mixed` an enum is its integer — and is struck as prose.
- **`crates/nvs-runtime/src/routes.rs` gap 1** — measured on `benches/serve-proxied.json` first; the trie
  is built only if the linear walk shows, and otherwise the gap becomes a stated bound with the figure's
  home named.
- **`crates/nvs-runtime/src/metrics.rs` gap 1** — the `otlp` pusher behind `[metrics] endpoint`, goal
  `m7-server-surface`'s stage 10, which that goal retired without building. No sentence, because none was
  needed: the gap says what closes it.

## Stage 3 — the checker and the front end

One file set: `crates/nvs-types/src/`, `crates/nvs-hir/src/`, `crates/nvs-syntax/src/`,
`crates/nvs-diagnostics/src/`.

- **`crates/nvs-diagnostics/src/embedded.rs` gap 1** — `autoload` roots are resolved into the bundle at
  build time, so a bundled program behaves like the tree it was built from.
- **`crates/nvs-hir/src/hierarchy.rs` gap 1** — `nvs-hir` is handed a roster of `Core` names at
  construction, so every link error comes from one pass.
- **`crates/nvs-hir/src/requires.rs` gap 1** — literal concatenations and `const`s in a `require` path
  are folded before the graph walk.
- **`crates/nvs-syntax/src/casing.rs` gap 1** — a `type` alias is PascalCase like a class. This is a rule
  change: `rule:types/type-alias`'s fragment is amended in the same slice, and no record opens.
- **`crates/nvs-syntax/src/lib.rs`**: gap 1 keeps the alias form and gives the targeted error; gaps 2
  and 3 are M1's two — `use function` / `use const` get the refusal naming
  `rule:classes/no-free-functions-or-constants`, and keyword-spelled segments past the first are covered
  by conformance cases rather than spot-checked.
- **`crates/nvs-types/src/defaults.rs` gap 1** — the call-site emitter carries the parameter's own IR
  type, so the default is allowed.
- **`crates/nvs-types/src/response.rs` gap 1** — the run-time default stays (`echo` is `text/html`, the
  last body member wins) and is struck as the module's own prose.

## Stage 4 — the library

One file set: `crates/nvs-stdlib/src/`.

- **The prepared-pattern channel**, one build for two items: `cldr.rs` gap 1 and `time.rs` gap 1. A
  literal pattern is prepared at compile time through a channel from the checker to the lowering; this is
  the goal's one ADR slot.
- **Builds to their sentence**: `ast.rs` gap 1 (a node carries a position — line, column, offset — and no
  text); `command.rs` gap 1 (the help renderer reaches the handler's signature at render time);
  `json.rs` gap 1 (the descriptor stays and widens) and gap 2 (the encoder walks an explicit heap stack
  charged to the request, so depth is always a catchable throw); `lib.rs` gap 1 (both registry gates
  widen past § 12, seeded by an outstanding-members ratchet file); `mime.rs` gap 1 (a shared `Ebml`
  case); `path.rs` gap 1 (a UNC root is a third root shape beside the drive letter); `queue.rs` gap 1 (a
  queue whose schema is behind is refused at boot) and gap 2 (the column arrives through `nvs queue
  migrate`'s converge, and `Core\Queue\Stats` gains its fifth counter — a spec § 6 amendment);
  `random.rs` gap 1 (`Core\Random\Seeded`, registered per spec § 11); `regex.rs` gap 1 (`qual_of` reads
  the parameter's declared `Qual` whatever its type); `test.rs` gap 2 (the embedding contract states it,
  and a `Ctx` asserts it when built); `uuid.rs` gap 1 (the bytes pair, a spec § 11 amendment); `xml.rs`
  gap 1 (a computed `namespaceUri()` that walks ancestor `xmlns` declarations); `zip.rs` gap 1 (Zip64's
  extra fields and end-of-directory record are read) and gap 2 (a CRC is verified by default and a
  mismatch refused).
- **Struck as stated bounds**: `cli.rs` gap 1 (a served request answers empty, neutral values, and the
  contract says so); `db/mod.rs` gap 1 (the defaults apply, and a deployment that wants bounds writes a
  block) and gap 2 (the split stands: a shape mismatch is a `ParseError`, as for `Json::decodeAs`);
  `path.rs` gap 2 (a drive-relative path stays one relative component, and the grammar says so);
  `test.rs` gap 1 (the runtime throw naming `assertEqualsDeep` stays).
- **M6's two**, `regex.rs` gaps 2 and 3: the step budget becomes a `[limits]` directive with today's
  constant as its default; the per-core compiled-pattern cache is charged to an accounting bracket, or
  replaced by the prepared-pattern channel above if that reaches regex literals too.
- **M7's and M8's four**: `html.rs` gap 1 and `response.rs` gap 1 are one gap seen from two sides — the
  sink in force selects a rendering, per spec § 3's table; `reflect.rs` gap 1 (the four classes § 1's
  roster still lacks) and gap 2 (a `protected` member reached reflectively from a subclass, on a second
  bit carried down from `nvs_types::layout`).

## Standing decisions

- **The user's rules, settled 2026-09-13**: every gap is closed or deferred to M9+, and a deferral is
  honest only when the item cannot be built without that milestone's work. **Code ahead of a decision
  wins; code behind one is a gap.** Where implemented, tested and verified code goes beyond or differs
  from a decision record or an earlier decision, the code counts: the rule fragment, plan or module doc is
  rewritten to match and the record stays frozen. Where the code lacks something a decision specifies,
  that is a gap to build — never a reason to rewrite the decision down to what exists. Where it is
  unclear which of the two it is, that is a `BLOCKED` for the user.
- **A `Decided:` sentence is not re-opened.** A session that finds the chosen option harder than the
  sheet priced builds it anyway, or records the obstacle in the handoff and takes the next item — never
  the other option silently.
- **An answer of "state it as a bound" strikes the gap**: the bound is written as the module's own prose
  (what it does, and the limit), the numbered item and its tag are deleted, and the rule fragment is
  amended if the rule promised otherwise. A bound is not a gap.
- **A decision whose answer changes a rule** amends that rule's fragment by its own process in the same
  slice, and opens no record: the sheet is the decision, and a rule's `because` may cite the gap it closed.
- **Three answers differ from the sheet's recommendation, deliberately**, and each is built as decided:
  `Core\Queue\Stats` gains a fifth counter (`queue.rs` gap 2, a spec § 6 amendment); `Core\Random\Seeded`
  is registered per spec § 11 (`random.rs` gap 1); the `uuid.rs` bytes pair is a spec § 11 amendment.
- **A gap leaves the register one of three ways and no fourth**: built and its item deleted; struck as
  prose; re-tagged to a milestone at M9 or later whose plan file states the scope, which `python
  tools/owners.py --deferrals` proves. Re-tagging to this goal, or to any goal, is not one of them: the
  gate is `--closes`, and the driver asks it of every goal at its end.
- **ADR slots**: one new record, and no other number — for the prepared-pattern channel from the checker
  to the lowering, when stage 4 builds it.
- **What it spends** is decided item by item and written in each module doc per
  `rule:programs/memory-priority`; the sheet's options already priced it. The collector and the JSON
  heap stack each state a per-request figure's home.
- **Not this goal**: the terminal gate (goal `gap-zero`); anything a milestone at M9 or later owns.
