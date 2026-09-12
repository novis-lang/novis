---
milestone: post-parity
---
# Loop goal 51 — the gap register is derived, empty, and the index that held it is gone

Every gap this repository records is either **closed** or **named to an uncarried milestone whose plan
states the scope** — and because this is the last hand-written entry on the chain, there is no third
answer left. `docs/agent/carried-gaps.md` is deleted when it is green, not because the gaps stopped
existing but because it has nothing left to index: the owner lives with the gap, and what is owed is
derived rather than copied.

**It sits last of the hand-written entries, in front of the dossier**, because a register can only be
emptied after everything that would add to it has run. Goals `carried-gaps` and `gap-owners` built the two gates; goal `unowned-sweep`
closed five items; every entry from 23 to 37 closes more. Emptying the register before them would be
emptying it twice.

Goal `outbound-proxy`'s whole acceptance list is this goal's floor, and it is traded exactly once —
[§ *Standing decisions*](#standing-decisions) names the check and why.

## Why here

**It stays last of the hand-written entries whatever else is scheduled**, directly in front of the
dossier: a register can only be emptied once everything that would add to it has run. A number is a
position, so a goal inserted ahead of this one renumbers it rather than colliding with it, and
`chain.py` is what keeps that true. It is the one entry that trades a floor check, replacing
`chain.py --check`'s assertion that the index exists with the stronger gate over five registers; its
goal prose names that trade and is the only place it is authorized.

## What "no open gaps" means here, precisely

Not "nothing is left to build". A milestone that has not been carried is scheduled work, and conflating
scheduled work with an unclosed gap is what made the roster of 110 look alarming — goal `gap-owners` argues that
at length and it is not re-argued. What this goal ends is the third state:

| State | After this goal |
|---|---|
| Owned by a live goal | **None.** Every hand-written entry is retired by then, so an item naming a goal is an item about to be orphaned by the exact mechanism this goal exists to end. |
| Owned by an uncarried milestone | **The only surviving state.** `M9`–`M12`, `M15`, `M16`, each with the scope written in its own file under [docs/plan/](../../plan/). |
| `unowned` | **Retired as an owner kind.** It was legitimate while entries were left to schedule; it is not once there are none. |
| Untagged | Already a failing check — goal `gap-owners`'s gate. |

So the acceptance property is stronger than "every item names an owner": **no item names a goal**, and
the dossier's generated proof entries are the one exception, named rather than inferred.

## Stage 0 — the catch-up

1. **Thirteen `carried-gaps.md` § *Owned* rows name a goal that retired without closing them** —
   `docs/agent/carried-gaps.md:48-59` and `:69`, under retired owners 21, 17, 13, 23 and 20. That file's
   own contract says to **strike the owner, not the entry**, and `python tools/playbook.py --check`
   already prints all thirteen. Strike them first, so stage 3 derives against a file in the state its
   contract describes rather than one already failing its own check.
2. **A row the tree already closed is struck, not re-pointed**, and the `{timeout?: Duration}` row is
   the worked example: the register carried it as "in neither registry row" while the module doc it
   pointed at said the option had landed and named the section that decided it, so goal `gap-owners`
   struck the row. The module doc is the home and the register is the copy. That is the rule this
   stage exists to fix the file with, and stage 3 applies it a dozen times.

## Stage 1 — the floor

Goal `outbound-proxy`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded for
anything above it, with the one named exception below.

## Stage 2 — the keystone: one register, two owner kinds, derived

Goal `gap-owners` builds `tools/owners.py` over the module docs' `# Known gaps` blocks. **This goal does not
build a second tool** — a second index maintained beside the first is the failure goals `carried-gaps` and `gap-owners` both
exist to end. It does three things to the one that exists.

1. **`owners.py` reads every register, not only the module docs**
   (`tools/owners.py`, `tools/holes.py:@main` for the derivation shape it already follows). Six are on
   the tree by then: the module-doc gap items; `nvs-ir`/`nvs-codegen`'s refusal sites via
   [carried-refusals.md](../carried-refusals.md); the four `crates/nvs-stdlib/tests/*-outstanding.txt`
   ratchets and their `# <owner>` columns; `carried-gaps.md`'s two sections;
   [guard-name-debt.md](../guard-name-debt.md); and the playbook's `[until:]` bullets. The item is that
   **"is anything open?" gets one answer and one exit code**, because six registers with six answers is
   how a gap goes missing between them — which is how `§18 stream` came to be filed under a goal that
   had already closed.
2. **`unowned` stops being an owner kind, and the gate becomes two.** A **live `[[goal]]` in
   [the goals directory](README.md)**, or an **uncarried milestone**. Nothing else parses, and there is no
   allowlist — goal `gap-owners`'s rule, applied at the end of the chain rather than in the middle.
3. **A deferral is earned and the gate says so.** The milestone must exist under [docs/plan/](../../plan/),
   must be **uncarried** — `python tools/plan.py --check` derives each milestone's `Carried by` cell from
   the chain, and a milestone with a cell is one whose goals have already walked — and its own file must
   state the scope that covers the item. **Deferring to a carried milestone is precisely the orphaning
   this goal ends**, so it fails rather than warns.
4. **The terminal gate**: `owners.py --check` exits non-zero on any item whose owner is a goal. That is
   the check the acceptance list runs, and it is what makes the goal's claim self-verifying rather than
   a session's opinion.

## Stage 3 — the re-derivation: what has closed, and what nobody recorded

**Nothing is carried in.** Every item is judged against the code as it stands, which is the whole reason
this stage exists rather than a list being worked down.

1. **A row struck carries the `file:line` that closed it.** AGENTS.md rule 7 is the standard, and a
   struck row with no evidence is this goal's own failure mode one level up.
2. **An item nothing had recorded is added.** Two are known at authoring time, both found by hand
   against goal `database`'s own acceptance list, and finding them by hand *is* the argument for stage 2:
   - **`Core\Queue` runs on three of five backends.** `crates/nvs-stdlib/src/queue.rs:1671`'s `runs`
     answers `false` for `Driver::Sqlite` and `Driver::SqlServer`, `nvs queue migrate` refuses both, and
     that module's gap 5 states it — indexed in no file. `rule:concurrency/enqueue-commits-with-your-write`
     is goal `database`'s stage 8 and asked for it whole.
   - **`Core\Db::stream` lands on PostgreSQL alone.** `crates/nvs-stdlib/src/db/stream.rs:@unstreamed`
     throws on the other four; `carried-gaps.md:50` indexes `stream`/`streamAs` under a retired owner and
     does not say *four of five drivers*, so the row understates its own scope. `serverVersion` is owed
     on all five for a second reason the same gap names — no driver keeps the server's version string.
3. **The register as the tree states it today**, with anchors, so a session spends no `grep` rediscovering
   them. **Stage 2's roster is the authority**; this is what was true on the day the goal was written, and
   an entry here that has since closed is *struck with its evidence*, never worked:

   | Item | Anchor |
   |---|---|
   | `nvs check` builds no grants, so the host diagnostic fires for nobody | `crates/nvs-types/src/intrinsics.rs` gap 6 |
   | A cycle closed only through an `array<T>` survives the teardown sweep | `crates/nvs-runtime/src/object.rs` § *The five walks* |
   | `queryAs<T>`'s three refusals are at run time, not compile time | `crates/nvs-stdlib/src/db/mod.rs` gap 4 |
   | `scope = "fleet"` parses, boots and is not armed | `crates/nvs-server/src/schedule.rs` § *What is not armed* |
   | **Struck** — the computed `$reason` *is* refused: `crates/nvs-types/src/reasons.rs:116` emits `E0805` and `crates/nvs-types/tests/tainted.rs:304` pins it | `crates/nvs-stdlib/src/html.rs` § *Known gaps* |
   | Twenty CLDR plural rosters throw; ordinals absent; eight pattern letters refused | `crates/nvs-stdlib/src/cldr.rs` gaps 2–4 |
   | `Core\Request::clientIp`/`host`/`scheme`, `Response::html`/`sendFile` | `crates/nvs-stdlib/tests/spec-members-part-two-outstanding.txt` |
   | `Core\Test::request`'s shape against spec § 13's one English cell | `docs/rules/testing/in-process-request.md` |
   | **Struck** — none of the syntax four was owed: `var` is built (`crates/nvs-syntax/src/parser/stmt.rs:229`) and PHP's property `var` is `E0122`; a grouped `use` is refused on purpose and a `goto` label is left unparsed behind `goto`'s own refusal (`crates/nvs-syntax/src/lib.rs` § *Deliberately rejected*); a keyword-spelled enum case is a case, pinned by `tests/conformance/reject/an-enum-case-spelled-as-a-lowercase-keyword-is-told-its-casing.nvst` | `crates/nvs-syntax/src/lib.rs` § *Known gaps* |
   | `nvs serve` runs on one core | `crates/nvs-cli/src/serve.rs:42` |
   | The driver matrix has no socket leg, so `AF_UNIX` is asserted against no real server | `crates/nvs-db/src/matrix.rs` gap 1 |
   | `Core\Metrics` — the exporter ships and no class row exists to read it through | `crates/nvs-stdlib/src/registry.rs` |
   | `Core\Process::spawn` — `run` is registered and is the whole of what there is | `crates/nvs-stdlib/src/process.rs` |
   | `rule:routing/a-shared-name-is-one-endpoint-everywhere` is shipped and unguarded | no `.nvst` declares two `#[Route]`s sharing a name |

## Stage 4 — the deferrals, each one earned

Every item that **cannot be built until a whole milestone's work exists** takes that milestone's tag, and
the milestone's file gains the scope sentence where it does not already carry one. Known at authoring time:

- `crates/nvs-cli/src/bundle.rs`'s `.nvsx` embedding → **M9**, the extension system.
- `nvs_safepoint`'s `DEBUG_BREAK` flag, cleared and ignored → **M10**, which carries `nvs dap`.
- `crates/nvs-runtime/src/decimal.rs`'s inlining and `crates/nvs-runtime/src/lib.rs`'s string fast path
  → **M12**, the optimising tier.

**One known item looks like a deferral and is not**, and it is the worked example of the rule biting:

- **The in-flight cycle collector** is a **decision, not a gap.**
  `rule:security/arena-is-an-ownership-root`'s record says outright that it remains open. It leaves the
  register for that rule's own fragment, which is where an open decision belongs; the register counts what
  is *owed*. `rule:routing/a-shared-name-is-one-endpoint-everywhere`'s missing fixture is the mirror case
  and goes the other way — a shipped rule with no proof is goal `dossier`'s shape, and it is named there.

## Stage 5 — the closures: `Core\Db` and `Core\Queue`

One group, one file set: `crates/nvs-stdlib/src/db/`, `crates/nvs-stdlib/src/queue.rs`, `crates/nvs-db/src/`.
These are goal `database`'s own stages 3 and 8, and they are first because they are the largest single thing the
register still owes.

1. **`stream` answers on every driver, or refuses per driver in a record** —
   `crates/nvs-stdlib/src/db/stream.rs:@stream_step`, `:@unstreamed`, `rule:core-classes/db-statement-members`.
   § 4's promise is constant memory and a *uniform* busy rule, so buffering behind the caller's back is not
   an answer and that module's gap 5 already argues why. What a driver needs is read state parked off the
   borrow, the way `nvs_db::PgCursor` is.
2. **`streamAs` is that class at a written type**, exactly as `queryAs` is `query`'s —
   `crates/nvs-stdlib/src/db/registry.rs:346`.
3. **`serverVersion` on all five**, which needs each connection to keep what it already parses and throws
   away: PostgreSQL's `server_version` `ParameterStatus`, MariaDB's greeting, TDS's `LOGINACK`, SQLite's
   library version, and MySQL's `(u16, u16, u16)` that exists for capability decisions.
4. **`Core\Queue` gains its SQLite dialect** — `crates/nvs-stdlib/src/queue.rs:@runs`, `:@no_dialect`.
   That module's gap 5 says SQLite is **one dialect away** and names the one construct it needs: its
   unique keys read nulls as distinct.
5. **SQL Server is a dialect *and* the vocabulary behind it**, and
   `rule:core-classes/queue-storage-is-a-table` orders the filtered index its nulls need before a fourth
   dialect is written. The rule is the design; a session takes it rather than re-deriving it.
6. **`queryAs<T>`'s three refusals move to compile time** — the same diagnostic band the CLDR and
   `html-to-source` items waited on, which goal `carried-gaps` opened.

## Stage 6 — the closures: everything the register still names

Grouped by file set rather than by topic, because the shared file set is what makes the second slice
cheap ([loop-authoring.md](../loop-authoring.md) § 7). The groups the register is expected to fall into:

1. **`crates/nvs-stdlib/src/registry.rs` + `process.rs`** — `Core\Metrics`' class row, `Core\Process::spawn`.
   Both are a registry entry over machinery that already ships.
2. **`crates/nvs-stdlib/src/cldr.rs`** — the plural rosters, the ordinals, the eight pattern letters.
   One diagnostic band, three items. `html.rs` left this group with the register row above it: its
   refusal is landed, and what that module still waits on is M7's HTML response rather than a band.
3. **`crates/nvs-db/src/matrix.rs` + `tests/db/compose.yaml`** — the socket leg, so `AF_UNIX` is asserted
   against a real server rather than against nothing.
4. **`crates/nvs-types/src/intrinsics.rs` + `crates/nvs-server/src/schedule.rs`** — the grants `nvs check`
   never builds, and the fleet lease that parses and is not armed.
5. **`crates/nvs-cli/src/serve.rs`** and the `*-outstanding.txt` request members, each against the entry
   that was meant to carry them — goals `per-core` and `test-request` both retired without doing so, and re-deriving is how
   this stage finds out whether the work landed under a different name.

## Stage 7 — the file is deleted

`carried-gaps.md` has nothing left to index, and a file kept for the shape of it is a file that fills up
again. It goes, and **every reader is re-pointed in the same slice** — a dangling pointer here is the
failure this goal is about:

- `tools/playbook.py:123`'s `CARRIED_GAPS` and its `--check` section
- `tools/chain.py --check`, which asserts the file's existence
- `docs/agent/carried-refusals.md:10`, which calls it its sibling
- [loop-authoring.md](../loop-authoring.md) § 8, whose destination for a gap found off the path becomes
  **the module doc that owns the code, with an owner tag**
- [session-prompt.md](../session-prompt.md):120, the same sentence in the handoff's contract
- `crates/nvs-stdlib/src/lib.rs:99` and
  `crates/nvs-stdlib/tests/spec-classes-part-two-outstanding.txt:11`
- goal `gap-owners`'s `every 'unowned' tag has a reason bullet behind it` check, which stage 2 has already
  replaced

## Stage 8 — it stays true

1. **`verify.py` runs the terminal gate**, so a gap added with a goal owner, or with no owner, fails
   before it is committed.
2. **`brief.py --where` routes to the register**, and the orientation pack carries the count so a session
   sees the number without reading anything.
3. **The tag survives a goal switch**, which is the property that makes a module doc the right home and
   an index the wrong one — `tools/goal-switch.py` carries checks forward and item lists not at all.

## Standing decisions

- **The floor is traded exactly once, and this is the trade.** The floor check *the durable gap list
  exists and every entry names an owner* (`python tools/chain.py --check`) asserts the existence of a file
  this goal deletes. It is **replaced, not dropped**, by the stage 2 gate, and the replacement is strictly
  stronger: it reads six registers where the old one read one, and it fails on a state the old one
  accepted. [loop-authoring.md](../loop-authoring.md) § 6 says the floor is never traded; this is the one
  exception on the chain, and it is written here so no session has to decide whether it may.
- **One new record and no other number**, for stage 5's streaming read across the five drivers.
  `rule:core-classes/db-one-api` specifies § 4's *behaviour* and deliberately does not specify how a
  driver parks read state, which is exactly the shape of an ADR slot. **The safe fallback is in the
  record, not in the code**: a driver whose protocol cannot park a cursor gets a *recorded refusal naming
  the driver*, and never a buffer behind the caller's back — that breaks constant memory and § 4's
  uniform busy rule at once. Stage 6 opens no number.
- **A deferral is never a way to finish this goal.** The gate catches the mechanical half — the milestone
  exists, is uncarried, and its file states the scope. The judgement half is this: a milestone tag is
  right only when the item **cannot be built** until that milestone's work exists, never when it is merely
  large. Low-to-medium effort is the bar, and an item that clears it is closed here.
- **An item that can be neither closed nor honestly deferred is a `BLOCKED` naming the item**, and it is
  the one thing worth blocking on in this goal. The run holds, the user schedules it, and the session
  presses on. Inventing an owner would leave the register in exactly the state — a list nobody trusts —
  that four goals have now been spent getting out of.
- **Ambiguity between a gap and a decision resolves toward *decision*, and it leaves the register.**
  Goal `gap-owners` decided this; it is not re-argued. A `# Known gaps` block holding settled non-goals is a block
  nobody reads.
- **A gap found off the path goes in the module doc that owns the code**, tagged, and never into a new
  index. If no module owns it, it is not a gap — it is a feature request, and it is the user's.
- **A module doc is rewritten as a whole when its gap closes**, never amended beside the old sentence
  (AGENTS.md rule 6). Thirteen of these docs state a blocker that this goal lifts, and a doc that reads as
  a changelog is the one output this goal must not produce.
- **What this spends**, per `rule:programs/memory-priority`: nothing at run time for the register, which is
  one tool invocation in `verify.py` over doc comments. Stage 5 spends **per streaming connection** what
  PostgreSQL already spends — read state parked off the borrow — and *reduces* what a large read holds on
  four drivers, since the alternative in place today is `query`'s whole result set. The queue's new
  dialects are texts, and spend nothing per request.
