---
milestone: M10
position: last
---
# Loop goal — There are no decision records: a rule says what is true, why, and what was declined

Every decision record is merged into the rules and deleted, and nothing can bring one back. That covers
`docs/decisions/NNNN.md`, `data/decisions/`, the generated `docs/decisions.md`, `docs/adr/README.md` and
`docs/adr/tooling-parity.md`. A record is frozen when it is written, so it goes stale as the rules move
on. The rules, the plan and the code are kept current, and they win wherever the two disagree. The
reasoning that still matters moves into the rule it explains, and every alternative that was declined
moves into that rule's **Declined** list. Everything else in a record is history, and `git log` keeps
it. When the goal is done, a new decision becomes a goal right away or goes into the plan. When it
lands, its reasons go into its rule.

A rule fragment after the merge, from `rule:routing/a-match-is-not-invocable` and record 0199:

```markdown
- **`access()` returns the decision's name**, such as `App\Role::Admin`, never the constant's value.
  The compiler checks that the name resolves and promises nothing about what it means. `null` comes
  only from a table built by hand, and a dispatcher treats it as denied. …
- **Memory:** each read of a match costs one string (the length of the decision's name) and one
  integer. A program that never reads a match pays nothing.

**Declined:**
- **The server calls the matched method.** Calling it would be dispatch
  (`rule:routing/the-servers-match-dispatches-nothing`).
- **`params` narrowed to a shape per route by comparing `name`.** The checker cannot narrow that
  way, and it would save only one `as`. …
```

The rule's JSON loses `because`, and the record's three test paths go into its `guardedBy`. The record's
story ("a user's report asked …"), its scope and dependency lines, its option numbering and its
verification prose are dropped.

## Why here

The user asked for it on 2026-10-08 and put it directly in front of goal `class-scoped-enums`, so behind
goal `ldap`, the live goal. Goals `class-scoped-enums`, `pdf` and `spreadsheet` each write a record in
their Stage 2 and read record sections through `[context] adrs`. If this goal lands first, none of them
writes a record that this goal would then have to merge. Stage 3 rewrites those three goals so that
they write rule fragments instead.

Goal `ldap` is left alone. It is live, and it may write more records before it is reached. Every record
on disk when a stage runs is in that stage's scope, so later records fall into Stage 10's range.

It carries `position: last` because every goal on the chain is pinned there.

## The size of the work

These are counts from the tree on 2026-10-08, made with `git grep` and a `bun -e` count:

- **275 records**, 61,019 lines, plus `docs/adr/` (480 lines) and `docs/decisions.md` (1,706 lines,
  generated from `data/decisions/`). Two records are `retired`: 0025 and 0089.
- **1,036 rules** in 21 topics, and every one of them names at least one record in `because`. Every
  record is named by some rule, so no record is unreachable.
- **About 600 files** cite a record outside the record set, in about 1,700 hand-written lines. The
  1,050 "Decided in" lines in the generated chapters disappear when `nv rules --render` stops writing
  them. Most citations are in `crates/` (273 files), `tests/conformance/` (131), `examples/` (58),
  `benches/` (25) and `Cargo.toml` (78 lines).
- **Declined reasoning lives inside accepted records.** 251 records have `## Alternatives rejected`
  and 78 have `## Options considered`, about 6,100 lines together. No record has a `rejected` status.

The merge goes **record by record, not rule by rule**. Each record is read once and its content goes
out to every rule in its `because`. Fragments are small (18,784 lines for all of them), so editing
several is cheap. A record that is read once per rule would be read up to sixty times.

## Stage 1 — the floor

Whatever the chain carries when this goal is reached. Every check that goal `ldap` turned green stays
green.

## Stage 2 — the new rule shape, the merge tool and the pilot

**Does:** Defines what a merged rule looks like, builds the tool that tracks the merge, deletes the
summaries, and merges record 0199 as the worked example.

One file set: `docs/agent/conventions.md`, `tools/nv/cmd/records.ts`, `tools/nv/cmd/rules.ts`,
`tools/nv/schema/rule.ts`, `tools/nv/cmd/decisions.ts`, `tools/nv/main.ts`, `tools/nv/test/`,
`docs/rules/routing/`, `data/rules/routing/`, `docs/rules/security/`, `data/rules/security/`.

- **The shape.** `conventions.md` § *A rule fragment* (`:395`) gains how a merged reason is written and
  the **Declined:** block: a bold lead line `**Declined:**`, then one bullet per declined alternative. A
  bullet opens with the declined thing in bold and gives its reason in one or two sentences. A memory
  cost is a `**Memory:**` bullet. Something declined that no rule covers becomes a rule whose title says
  it does not exist, the way `rule:routing/no-wildcard-verb` already does.
- **`because` may be empty while the merge runs.** `schema/rule.ts:22-26` and `rules.ts:183-186` stop
  refusing an empty `because`. A rule's last record number is removed when that record is merged.
- **`bun nv records --left [<from>-<to> | adr]`** prints the records still on disk in that scope. They
  are grouped by the topic most of their rules sit in, and each one shows its rule ids and how many files
  cite it. The last line is `<n> left in <scope>`: `0002-0040`, say, or `docs/adr` for `adr`.
- **`bun nv records --check`** also fails on a citation of a record that is not on disk. It prints
  `no citation names a missing record` when there is none. It uses the `DECISION` pattern in
  `tools/nv/lib/prose.ts:92-113`. That pattern must not match RFC citations such as `3986 §`, so a bare
  section cite matches only `\b0[0-9]{3} §`.
- **The summaries go.** `tools/nv/cmd/decisions.ts`, its `main.ts:37` entry, `data/decisions/`,
  `docs/decisions.md` and `docs/agent/decisions-summary.md` are deleted without being merged. Each summary
  restates a record, so the records are the source.
- **The pilot: record 0199**, merged into `routing/matching-is-not-dispatching`,
  `routing/a-match-is-not-invocable` and `security/access-is-checked-for-presence-not-meaning`, exactly as
  the example at the top of this file shows. Its citations elsewhere are rewritten, then it is deleted.
- **Pinned by** the Stage 2 checks.

## Stage 3 — the queued goals stop writing records

**Does:** Merges the records that the queued goals read, and rewrites those goals so that they write
rule fragments.

One file set: `docs/agent/goals/class-scoped-enums.md`, `pdf.md`, `spreadsheet.md`, their
`data/goals/*.json` and `*.handoff.json`, and the rules the records below reach.

- **The records first:** 0051, 0120, 0121, 0123, 0128, 0166, 0190, 0247. Every `[context] adrs` slice of
  the three goals names one of them. Each is merged the Stage 4 way. Content that a slice prints and that
  no rule holds yet goes into a rule, for example 0128's *Verification* corpus.
- **Then the goals.** Every `adrs` entry becomes the rule id that now holds its content. Each Stage 2 that
  says "the record" writes the fragments instead, with the reasons and the Declined list inside them.
  "One record slot" and every "amends ADR NNNN" sentence go. "A decision record" leaves every `shapes`
  list. The tradeoffs each goal states stay in its prose, and the fragment and the commit carry them.
  `class-scoped-enums`' target `docs/adr/README.md:342` becomes the rule that Stage 11 gives that
  paragraph.
- **Pinned by** the Stage 3 checks.

## Stages 4 to 10 — the merge, by record number

**Does:** Merges every record in the stage's range into its rules, rewrites every citation of it, and
deletes it. Stage 4 covers 0002–0039, Stage 5 0040–0079, Stage 6 0080–0119, Stage 7 0120–0159, Stage 8
0160–0199, Stage 9 0200–0239, and Stage 10 everything from 0240 on. Each range is one `git` glob, such as
`docs/decisions/00[4-7]*`, so its check does not depend on a tool that Stage 12 deletes.

The file set of one slice is the records `bun nv records --left <range>` groups under one topic, that
topic's `docs/rules/<topic>/` and `data/rules/<topic>/`, and the files that cite those records. One
slice for each record, in this order:

1. **Read the record once** (`bun nv peek docs/decisions/NNNN.md`), and next to it the fragment of
   every rule whose `because` names it.
2. **Keep what is still true and not already in the rule.** That means why the rule holds, what it
   spends in memory, the bounds and limits it sets, and every alternative it declined with the reason.
   Write each one into the rule it belongs to. A record of a project-wide choice, such as 0004 (the
   priority ordering), goes into the rule nearest to it. A choice about how a crate is built, such as a
   dependency or a vendored patch, goes into that crate's module doc, with its own **Declined:** lines.
3. **Drop the rest.** That means the story, who asked, the dates, the scope and dependency lines, the
   option numbering, what the rule used to say, and anything the rule or the code now contradicts. The
   code and the tests win over a record. A record's claim that the tree does not hold is not merged as true. It becomes a gap under `data/gaps/`
   when it is still wanted, and it is dropped when it is not.
4. **Move its test paths** from *Validated by* and *Verification* into the `guardedBy` of the rule they
   test.
5. **Rewrite every citation of it** in the same slice: `git grep -n -E 'ADR NNNN|decisions/NNNN|\bNNNN §'`.
   A code comment cites the rule (`rule:<topic>/<slug>`) or states the reason in place. A comment an end
   user reads loses the number and follows AGENTS.md § *Text an end user reads*. A test that reads the
   record file is moved off it. `crates/nvs-stdlib/src/reflect.rs:4883-4925` reads 0019 § 1, so the
   slice that merges 0019 moves that list into the test or into a rule it can read.
6. **Remove its number** from each rule's `because`, delete the record, and run `bun nv rules --render`.

A retired record (0025, 0089) gets the same steps. Its "why this is retired" text is history, and only
what is still true or still declined is kept.

- **Pinned by** each stage's check: `git ls-files --error-unmatch` on the range's glob fails, because
  no record in the range is left in the index. Stage 10's check is the whole `docs/decisions/`
  directory.

## Stage 11 — the start-of-project decisions

**Does:** Merges `docs/adr/README.md` and `docs/adr/tooling-parity.md` and deletes `docs/adr/`.

One file set: `docs/adr/`, the rules its paragraphs reach, and the about 40 files that cite
`adr/README` or `adr/tooling-parity`.

- Each `README.md` paragraph goes into the rule it decides, or becomes a rule when none exists.
  `README.md:342` ("declared at file scope, or not at all") goes to the `types` or `classes` rule that
  `class-scoped-enums` amends. `tooling-parity.md`'s declined `nvs fix` becomes a Declined line.
  `docs/future-ideas.md` is the user's own file and stays as it is.
- **Pinned by** `git ls-files --error-unmatch docs/adr` failing.

## Stage 12 — the tools forget records

**Does:** Removes every piece of tooling that reads, renders, numbers or slices a record.

One file set: `tools/nv/cmd/`, `tools/nv/schema/`, `tools/nv/lib/`, `tools/nv/renderers/`,
`tools/nv/test/`, `data/schema/`, `website/config/site.mjs`, `website/src/lib/core.ts`,
`.github/workflows/ci.yml`.

- **`because` is gone** from `schema/rule.ts:4,15,22-26` and from all 1,036 rule records.
  `schema/decision.ts` and its `index.ts:11,32` entry go. `nv render` stops writing
  `data/schema/decision.schema.json`.
- **`nv rules`** stops writing the "Decided in" trailer (`rules.ts:71`, `:239`) and printing `because:`
  (`:421`).
- **`nv orient`** loses the `adrs` field (`schema/goal.ts:14`, `orient.ts:331-365`, `:773-799`,
  `:286`), the record-number form of `rules` (`:719-753`, `:1582-1592`), "decided in" (`:764-765`),
  `runNumbers` (`:1345-1398`) and the "A decision record" pairing in `SHAPE_IMPLIES` (`:103`).
  `data/schema/goal.schema.json` and `side_goal.schema.json` are rendered again.
- **`nv session`** stops counting records (`session.ts:223-229`, `:253`, `:1064-1067`, `:1118`, `:1159`,
  `:1631`) and gating on them (`:504-510`, `:1053`). **`nv chain`** loses the `docs/decisions/`
  exemption (`chain.ts:25`, `:77-81`). **`nv audit`** loses `HISTORY` and `RECORD_HISTORY`
  (`audit.ts:18-19`, `:193-195`, `:264-269`) and adds `docs/decisions/`, `data/decisions/` and
  `docs/adr/` to `LEGACY` (`:219`). **`nv loop-stats`** drops its `adr` category (`:98`, `:586-587`).
  **`nv reference`** drops `ADR_PAREN_RE` (`:80-81`, `:229`).
- **The plan field "ADR slices landed"** goes from `schema/plan-status.ts:15`, `plan.ts:35`,
  `orient.ts:111`, `docs/implementation-plan.md:38-40` and AGENTS.md's field list.
- **The website** loses `decisionRecord()` (`website/config/site.mjs`, `renderers/website-core.ts:116-124`)
  and the `adrs` list of a class (`website-core.ts:347`, `:381`, `:504`, `:641`, `website/src/lib/core.ts`).
  The pages that called it link to the rule or the guide that holds the reason.
- **The tests** that assert record behaviour are deleted or rewritten: `records.test.ts`,
  `orient.test.ts:43-66`, `prose.test.ts:17,48-50`, `audit.test.ts`, `chain.test.ts:147`.
- **`nv records` itself goes** (`records.ts`, `main.ts:61`). Its citation check moves to Stage 13's
  guard.
- **Pinned by** the Stage 12 checks.

## Stage 13 — the process docs, and the guard

**Does:** Rewrites every sentence that tells an agent to write, read, cite or number a record. Then it
adds the check that stops a record from coming back.

One file set: `AGENTS.md`, `docs/agent/*.md`, `docs/agent/goals/README.md`, `data/playbook/`,
`docs/rules/tooling/`, `data/rules/tooling/`, `docs/plan/`, `tools/nv/cmd/rules.ts`, `tools/nv/lib/prose.ts`.

- **AGENTS.md:** `:35`, `:41-43` (a fragment is the rule, a record is history), `:47-48` (ADR 0004
  becomes the rule that Stage 4 gave it), `:62-63` (memory is said in the rule or the doc comment),
  `:130`, `:154`, `:326`. Add one sentence to § *Where to look*: **"There are no decision records. A
  rule holds its reasons and its Declined list, and a new decision becomes a goal or goes into the
  plan."**
- **`conventions.md`:** § *A decision record* (`:474-556`) goes. So do the commit scope `adr` (`:27`),
  the record citation forms (`:69-70`, `:364-383`) and the record sentences in § *A rule fragment*
  (`:397-398`, `:411`, `:420`, `:425-427`).
- **The rest of `docs/agent/`:** `loop-authoring.md` (`:80-81`, `:83`, `:91-136`, `:157`, `:178`,
  `:209-212`, `:261-271`, `:336-337`), `session-prompt.md` (`:52-53`, `:83`, `:92`), `doc-style.md`
  (`:8`, `:13-31`, `:58`), `doc-cleanup.md` (most of it), `user-report.md` (`:114`, `:135-136`: a new
  decision is a goal or a plan entry), `commands.md` (`:83-85`, `:131`, `:293-302`, `:344`, `:517`),
  `goal-decisions.md` (`:8`, `:29`), `playbook.md:22`.
- **`goals/README.md`:** rule 2 (ADR slots, `:72-77`) goes, and rule 4's record exemption (`:84-86`)
  goes too. The table rows stop saying "one new record".
- **Rules about records:** `rule:tooling/the-chain-names-its-live-goal` (`:11-12`) and
  `rule:tooling/a-repository-fact-is-one-json-record` (`:1`, `:11`).
- **The playbook bullets about records** are deleted. They are traps of a workflow that no longer
  exists: `tooling/a-decision-records-changes-block-needs-modifies-spelled-out`,
  `a-new-rules-because-names-only-the-records-that-created-or`,
  `an-adr-cross-reference-can-name-a-section-that-says-nothing`,
  `a-context-adrs-gap-does-not-close-by-naming-the-section-in`,
  `writing-novis/an-adr-can-specify-a-diagnostic-code-that-is-already-taken`, and any other bullet that
  only makes sense with records.
- **`docs/plan/`:** the record citations in `design.md` (18), `m17` (8), `m4b` (7), `m8`, `m9` (6 each),
  `m4`, `m10` (5 each) and `m2` (1) were rewritten in Stages 4 to 11. This stage checks that none is
  left.
- **The guard, in `bun nv rules --check`**, which CI's `docs` job and every wrap already run. It fails
  when `docs/decisions/`, `data/decisions/` or `docs/adr/` exists, and when a tracked text file outside
  `vendor/` matches `ADR [0-9]{4}`, `decisions/[0-9]{4}`, `\b0[0-9]{3} §` or `docs/adr/`. A passing run
  prints `no decision record and no citation of one`. A new test file, `tools/nv/test/rules.test.ts`,
  shows each of those cases failing.
- **Pinned by** the Stage 13 checks.

## Standing decisions

- **The user's calls, as instructions.** Decision records are removed completely and never come back.
  The reasoning behind accepted and declined features and rules is not lost: it goes into the docs that
  already exist. Everything a record says that is described elsewhere already is deleted. A feature that
  is implemented is a rule, and a rule says what is implemented now and what was explicitly declined.
  It never says where the reasoning came from. A new decision becomes a goal right away, or it goes into
  the plan. The goal sits in front of `class-scoped-enums`.
- **The goal writer's calls, not confirmed by the user, also standing.**
  - The Declined list is prose inside the fragment, under a bold `**Declined:**` line. It is not a JSON
    field, so a rule's reasons and its declined alternatives are read together.
  - Something declined that no rule covers becomes a rule whose title says it does not exist.
  - Why a crate is built the way it is goes into the crate's module doc, never into a rule.
  - The summaries in `data/decisions/` are deleted without being merged.
  - The merge goes by record number.
  - The plan field "ADR slices landed" is removed.
  - The guard lives in `nv rules --check`.
  - The milestone tag is M10.
- **The code wins.** A record that disagrees with the code, a test or a rule loses. Its claim is
  dropped, or it is filed as a gap when it is still wanted. A merge never changes behaviour. A session
  that finds a real bug files it as a gap and goes on.
- **Merge only what is not there yet.** A sentence that only says again what the fragment already
  says is dropped. A fragment grows by its reasons and its Declined list, and never by a copy.
- **No new tool survives the goal.** `nv records --left` exists only to track the merge and is deleted in
  Stage 12. The guard is the only new code that stays. So no check of this goal calls `nv records`: the
  driver runs every check on every sweep, and a check that calls a deleted tool would turn red. The
  checks call `git`, `bun nv peek` and `bun nv rules --check`.
- **`.claude/settings.json` is not edited by a session.** The user removes the `bun nv decisions:*` and
  `bun nv records:*` allow entries (`:21`, `:44`, `:104`, `:127`) when this goal is reached.
- **Neutral names** in every rewritten line (AGENTS.md rule 11). A record that quotes a report keeps
  none of its names when it is merged.
- **The tradeoffs**, as AGENTS.md asks. Language performance, memory and surface: no change. For agents:
  each reason has one home that stays current, and every declined alternative is printed with its rule,
  which stops the same proposal from coming back. A fragment grows by what it keeps, and `bun nv orient`
  prints a named rule whole, so a goal that names a rule pays for its Declined list. Sixty-one thousand
  lines of history leave the tree, and `git log` still has them.
