---
looper: 1
title: There are no decision records
summary: "Every decision record is merged into the rules and deleted, and nothing can bring one back."
context: [tools/nv/cmd/records.ts, tools/nv/cmd/rules.ts, tools/nv/schema/rule.ts, tools/nv/lib/prose.ts]
stages:
  - id: 2
    title: "the new rule shape, the merge tool and the pilot"
    status: done
    extra:
      context:
        adrs: []
        rules: [routing/matching-is-not-dispatching, routing/the-servers-match-dispatches-nothing, routing/no-wildcard-verb, security/access-is-checked-for-presence-not-meaning]
        shapes: [Citing a document]
  - id: 3
    title: the queued goals stop writing records
    status: done
    extra:
      context:
        adrs: []
        rules: []
        shapes: []
  - id: 4
    title: "the merge, records 0002 to 0039"
    status: done
    extra:
      context:
        adrs: []
        rules: []
        shapes: []
  - id: 5
    title: "the merge, records 0040 to 0079"
    status: done
    extra:
      context:
        adrs: []
        rules: []
        shapes: []
  - id: 6
    title: "the merge, records 0080 to 0119"
    status: done
    extra:
      context:
        adrs: []
        rules: []
        shapes: []
  - id: 7
    title: "the merge, records 0120 to 0159"
    status: open
    extra:
      context:
        adrs: []
        rules: []
        shapes: []
  - id: 8
    title: "the merge, records 0160 to 0199"
    status: open
    extra:
      context:
        adrs: []
        rules: []
        shapes: []
  - id: 9
    title: "the merge, records 0200 to 0239"
    status: open
    extra:
      context:
        adrs: []
        rules: []
        shapes: []
  - id: 10
    title: "the merge, records from 0240 on"
    status: open
    extra:
      context:
        adrs: []
        rules: []
        shapes: []
  - id: 11
    title: the start-of-project decisions
    status: open
    extra:
      context:
        adrs: []
        rules: []
        shapes: []
  - id: 12
    title: the tools forget records
    status: open
    extra:
      context:
        adrs: []
        rules: [tooling/a-repository-fact-is-one-json-record]
        shapes: []
  - id: 13
    title: "the process docs, and the guard"
    status: open
    extra:
      context:
        adrs: []
        rules: [tooling/the-chain-names-its-live-goal, tooling/a-repository-fact-is-one-json-record]
        shapes: []
checks:
  - id: decision-records-retired-the-pilot-rule-has-a-declined-list
    stage: 2
    name: "rulebook: the pilot rule carries its memory cost and its Declined list"
    run: [bun, nv, peek, "rule:routing/a-match-is-not-invocable"]
    output_ordered: ["**Memory:**", "**Declined:**"]
  - id: decision-records-retired-the-pilot-record-is-gone
    stage: 2
    name: record 0199 is merged and deleted
    run: [git, ls-files, --error-unmatch, docs/decisions/0199.md]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-the-summary-records-are-gone
    stage: 2
    name: the decision summaries under data/decisions are deleted
    run: [git, ls-files, --error-unmatch, data/decisions]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-the-summary-page-is-gone
    stage: 2
    name: the generated docs/decisions.md is deleted
    run: [git, ls-files, --error-unmatch, docs/decisions.md]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-nv-decisions-is-gone
    stage: 2
    name: "the `nv decisions` command is deleted"
    run: [git, ls-files, --error-unmatch, tools/nv/cmd/decisions.ts]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-the-queued-goal-records-read-no-record
    stage: 3
    name: "the queued goals' records slice no record and name no record shape"
    run: [git, grep, -n, -E, "adrs: \\[\"|A decision record", --, docs/goals/class-scoped-enums.md, docs/goals/pdf.md, docs/goals/spreadsheet.md]
    exit: nonzero
  - id: decision-records-retired-the-queued-goal-prose-writes-no-record
    stage: 3
    name: "the queued goals' prose cites no record and claims no record slot"
    run: [git, grep, -n, -i, -E, "ADR [0-9]{4}|record slot|docs/adr/|docs/decisions/", --, docs/goals/class-scoped-enums.md, docs/goals/pdf.md, docs/goals/spreadsheet.md]
    exit: nonzero
  - id: decision-records-retired-records-0002-to-0039-are-merged
    stage: 4
    name: records 0002 to 0039 are merged and deleted
    run: [git, ls-files, --error-unmatch, "docs/decisions/00[0-3]*"]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-records-0040-to-0079-are-merged
    stage: 5
    name: records 0040 to 0079 are merged and deleted
    run: [git, ls-files, --error-unmatch, "docs/decisions/00[4-7]*"]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-records-0080-to-0099-are-merged
    stage: 6
    name: records 0080 to 0099 are merged and deleted
    run: [git, ls-files, --error-unmatch, "docs/decisions/00[89]*"]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-records-0100-to-0119-are-merged
    stage: 6
    name: records 0100 to 0119 are merged and deleted
    run: [git, ls-files, --error-unmatch, "docs/decisions/01[01]*"]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-records-0120-to-0159-are-merged
    stage: 7
    name: records 0120 to 0159 are merged and deleted
    run: [git, ls-files, --error-unmatch, "docs/decisions/01[2-5]*"]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-records-0160-to-0199-are-merged
    stage: 8
    name: records 0160 to 0199 are merged and deleted
    run: [git, ls-files, --error-unmatch, "docs/decisions/01[6-9]*"]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-records-0200-to-0239-are-merged
    stage: 9
    name: records 0200 to 0239 are merged and deleted
    run: [git, ls-files, --error-unmatch, "docs/decisions/02[0-3]*"]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-every-record-is-merged
    stage: 10
    name: every record is merged and docs/decisions is gone
    run: [git, ls-files, --error-unmatch, docs/decisions]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-docs-adr-is-merged
    stage: 11
    name: docs/adr is merged into rules and deleted
    run: [git, ls-files, --error-unmatch, docs/adr]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-no-rule-has-because
    stage: 12
    name: "no rule record carries a `because` field"
    run: [git, grep, -n, "\"because\"", --, data/rules]
    exit: nonzero
  - id: decision-records-retired-the-record-schema-is-gone
    stage: 12
    name: the decision record schema is deleted
    run: [git, ls-files, --error-unmatch, tools/nv/schema/decision.ts]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-nv-records-is-gone
    stage: 12
    name: "the `nv records` command is deleted"
    run: [git, ls-files, --error-unmatch, tools/nv/cmd/records.ts]
    exit: nonzero
    output_ordered: [did not match]
  - id: decision-records-retired-no-schema-field-names-a-record
    stage: 12
    name: "no schema has the `adrs` context field or the ADR plan field"
    run: [git, grep, -n, -E, "adrs|adrSlicesLanded", --, tools/nv/schema]
    exit: nonzero
  - id: decision-records-retired-agents-md-says-there-are-none
    stage: 13
    name: AGENTS.md says there are no decision records
    run: [git, grep, -n, -F, There are no decision records, --, AGENTS.md]
    output_ordered: ["AGENTS.md:"]
  - id: decision-records-retired-the-guard-tests-pass
    stage: 13
    name: "the guard's tests show a record directory and each citation form failing the check"
    run: [bun, test, ./tools/nv/test/rules.test.ts]
    output_ordered: [" 0 fail"]
  - id: decision-records-retired-the-guard-is-green
    stage: 13
    name: the tree holds no decision record and no citation of one
    run: [bun, nv, rules, --check]
    output_ordered: [no decision record and no citation of one]
extra:
  milestone: M10
  files: []
  env:
    valgrind:
      skip: [examples/fatal.nvs, examples/uncaught.nvs, examples/cache-shared-socket.nvs, examples/queue-sqlite.nvs, examples/cache-shared-tls.nvs]
    wsl:
      targetDir: /var/tmp/nvs-target-wsl
  context:
    adrs: []
    milestones: []
    plan: [Open now, Blocking]
    playbook: ["Tooling > the end", "Tooling > one call reads", "Tooling > a commit message", "Tooling > another agent"]
    rules: [routing/a-match-is-not-invocable]
    shapes: [A rule fragment, A code comment, A commit message]
---

# There are no decision records

## Why

### The target

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

### Why here

The user asked for it on 2026-10-08 and put it directly in front of goal `class-scoped-enums`, so behind
goal `ldap`, the live goal. Goals `class-scoped-enums`, `pdf` and `spreadsheet` each write a record in
their Stage 2 and read record sections through `[context] adrs`. If this goal lands first, none of them
writes a record that this goal would then have to merge. Stage 3 rewrites those three goals so that
they write rule fragments instead.

Goal `ldap` is left alone. It is live, and it may write more records before it is reached. Every record
on disk when a stage runs is in that stage's scope, so later records fall into Stage 10's range.

It carries `position: last` because every goal on the chain is pinned there.

### The size of the work

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

### Stage 1 — the floor

Whatever the chain carries when this goal is reached. Every check that goal `ldap` turned green stays
green.

### Stages 4 to 10 — the merge, by record number

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

## Stages

### 2. the new rule shape, the merge tool and the pilot

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

### 3. the queued goals stop writing records

**Does:** Merges the records that the queued goals read, and rewrites those goals so that they write
rule fragments.

One file set: `docs/goals/class-scoped-enums.md`, `pdf.md`, `spreadsheet.md`, and the rules the
records below reach.

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

### 4. the merge, records 0002 to 0039

**Does:** Merges, re-cites and deletes every record in the range.

### 5. the merge, records 0040 to 0079

**Does:** Merges, re-cites and deletes every record in the range.

### 6. the merge, records 0080 to 0119

**Does:** Merges, re-cites and deletes every record in the range.

### 7. the merge, records 0120 to 0159

**Does:** Merges, re-cites and deletes every record in the range.

### 8. the merge, records 0160 to 0199

**Does:** Merges, re-cites and deletes every record in the range.

### 9. the merge, records 0200 to 0239

**Does:** Merges, re-cites and deletes every record in the range.

### 10. the merge, records from 0240 on

**Does:** Merges, re-cites and deletes every record left, including any that goal `ldap` wrote, and removes `docs/decisions/`.

### 11. the start-of-project decisions

**Does:** Merges `docs/adr/README.md` and `docs/adr/tooling-parity.md` and deletes `docs/adr/`.

One file set: `docs/adr/`, the rules its paragraphs reach, and the about 40 files that cite
`adr/README` or `adr/tooling-parity`.

- Each `README.md` paragraph goes into the rule it decides, or becomes a rule when none exists.
  `README.md:342` ("declared at file scope, or not at all") goes to the `types` or `classes` rule that
  `class-scoped-enums` amends. `tooling-parity.md`'s declined `nvs fix` becomes a Declined line.
  `docs/future-ideas.md` is the user's own file and stays as it is.
- **Pinned by** `git ls-files --error-unmatch docs/adr` failing.

### 12. the tools forget records

**Does:** Removes every piece of tooling that reads, renders, numbers or slices a record.

One file set: `tools/nv/cmd/`, `tools/nv/schema/`, `tools/nv/lib/`, `tools/nv/renderers/`,
`tools/nv/test/`, `data/schema/`, `website/config/site.mjs`, `website/src/lib/core.ts`,
`.github/workflows/ci.yml`.

- **`because` is gone** from `schema/rule.ts:4,15,22-26` and from all 1,036 rule records.
  `schema/decision.ts` and its `index.ts:11,32` entry go. `nv render` stops writing
  `data/schema/decision.schema.json`.
- **`nv rules`** stops writing the "Decided in" trailer (`rules.ts:71`, `:239`) and printing `because:`
  (`:421`).
- **`nv orient`** and **`nv pack`** lose the `adrs` field (`schema/goal.ts:14`, `pack.ts:236-276`,
  `:362-388`, `:221`), the record-number form of `rules` (`pack.ts:308-343`, `orient.ts:930-940`),
  "decided in" (`pack.ts:353-354`), `runNumbers` (`pack.ts:689-743`) and the "A decision record"
  pairing in `SHAPE_IMPLIES` (`pack.ts:46`).
  `data/schema/goal.schema.json` and `side_goal.schema.json` are rendered again.
- **`nv session`** stops counting records (`session.ts:117-123`, `:142`, `:829-833`, `:884`, `:925`,
  `:1299`) and gating on them (`:391-397`, `:819`). **`nv chain`** loses the `docs/decisions/`
  exemption (`chain.ts:25`, `:77-81`). **`nv audit`** loses `HISTORY` and `RECORD_HISTORY`
  (`audit.ts:18-19`, `:193-195`, `:264-269`) and adds `docs/decisions/`, `data/decisions/` and
  `docs/adr/` to `LEGACY` (`:219`). **`nv loop-stats`** drops its `adr` category (`:98`, `:586-587`).
  **`nv reference`** drops `ADR_PAREN_RE` (`:80-81`, `:229`).
- **The plan field "ADR slices landed"** goes from `schema/plan-status.ts:15`, `plan.ts:44`,
  `pack.ts:54`, `docs/implementation-plan.md:38-40` and AGENTS.md's field list.
- **The website** loses `decisionRecord()` (`website/config/site.mjs`, `renderers/website-core.ts:116-124`)
  and the `adrs` list of a class (`website-core.ts:347`, `:381`, `:504`, `:641`, `website/src/lib/core.ts`).
  The pages that called it link to the rule or the guide that holds the reason.
- **The tests** that assert record behaviour are deleted or rewritten: `records.test.ts`,
  `orient.test.ts:43-66`, `prose.test.ts:17,48-50`, `audit.test.ts`, `chain.test.ts:147`.
- **`nv records` itself goes** (`records.ts`, `main.ts:61`). Its citation check moves to Stage 13's
  guard.
- **Pinned by** the Stage 12 checks.

### 13. the process docs, and the guard

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

## Handoff

### State

**Stages 2 to 6 are done.** Every record from 0002 to 0119 is merged and deleted, one commit each. Records 0120, 0121, 0123, 0128, 0166, 0190, 0247 and 0199 went earlier; 34 are left in Stage 7's range (`bun nv records --left 0120-0159`). The rulebook counts 1036 rules (0098 added `rule:expressions/no-null-skipping-pipeline`).

Each merge goes to one general-purpose subagent, run one after another. The brief is conventions.md § *A decision record* plus the traps below; the subagent reads it from a file under `.agent-tmp/`, which the session writes and deletes. Two middle records took one subagent 120k to 200k, one wide record 115k to 130k, and one narrow record 75k. Every subagent reports `bun nv rules --render` exiting 1: that is the 16 queued-goal findings below, and the render still writes every chapter. Keep each `because` array well-formed: a `]` joined onto the line before, or the blank line `bun nv splice` leaves when it removes a whole line, makes the render drop rules silently, so `bun nv rules --list` must still count 1036 plus any added. A bun script under `.agent-tmp/` that removes the number only when re-serialising the original JSON reproduces it byte for byte is how every merge did it safely. `bun nv rules --render` can rewrite `data/schema/rule.schema.json` (its `because` loses `minLength` and `x-references`); it is restored uncommitted, so whether the committed schema is stale is still open. `website/src/data/core.json` lines that cite a record are hand-edited; never its full-render drift. Two middle records, or one record past 30 citing files, fit one subagent at 90k to 190k; three middle records reached 184k.

The brief must also say: a comment edit in a `.nvst` case keeps the case's line count, because its `--EXPECT--` names `case.nvs:LINE` (the 0072 merge broke `tests/conformance/core/a-task-map-and-a-task-all-refuse-each-others-subject.nvst` that way). A `.nvst` title line that cites the record loses only that citation, on the same line; a title is not a key. A record's sections are cited as a bare `§ N` in the modules whose head names it, and `records --check` does not see those: the merges map them to `rule:` ids. A runtime message, help string or test expectation that names a record is changed together with the case that expects it, in one slice by the session itself (the `#[Api]` help strings in `crates/nvs-types/src/routes.rs` were). A Rust `expect`/`panic!`/assert message is not an expectation, and the subagent may rewrite it.

`bun nv records --check` no longer reads `tools/nv/test/prose.test.ts`, whose fixtures name record numbers (`tools/nv/cmd/records.ts`, `PROSE_TEST`), and it is clean. A test in `crates/nvs-stdlib/src/reflect.rs` read record 0019 for its class roster and now reads `docs/rules/core-classes/reflect.md`: check `git grep -n "docs/decisions/"` in `crates/` and `tools/` before deleting a record a test may read. The `nvs-stdlib` LDAP tests raced on one OU name and missed a 5 s deadline under a full verify run; both are fixed in `crates/nvs-stdlib/tests/ldap.rs`.

The merge shape is in `docs/agent/conventions.md` § *A rule fragment* (reasons, a `**Memory:**` bullet, a `**Declined:**` list) and § *A decision record* (no new record; the five merge steps). `because` may be empty and must name a record on disk (`tools/nv/cmd/rules.ts:184-186`). `data/decisions/` went with the summaries, so the `decision` schema went too: `because` is a list of plain strings now, which pulls part of Stage 12 forward (`tools/nv/schema/decision.ts` is already gone). Record 0199 is merged into `rule:routing/a-match-is-not-invocable` and `rule:security/access-is-checked-for-presence-not-meaning`; `routing/matching-is-not-dispatching` already said everything 0199 added.

`bun nv records` is now only the merge tracker: `--left [<from>-<to> | adr]` and `--check` (the `links` check inside the records, and the tree-wide `citations` check). The old record audit is gone. A record left on disk that links to a merged one must be unlinked to plain text, which the `links` check reports; `docs/decisions/0255.md` was the one for 0199. Citations inside `docs/decisions/` and this goal's own file are not checked. A "cited by N file(s)" count in `--left` includes the generated chapter under `docs/rules/`, which drops out with the record's `because` entry.

`bun nv rules --check` is red before this goal touched it: 16 findings, all queued goals (`class-scoped-enums`, `pdf`, `spreadsheet`, `nvs-code`, `env-context`) citing rules they will create. Stage 13's guard check needs it green.

### Next

**Stage 7: records 0120 to 0159**, 34 left. One subagent per one to three records, run one after another, each committing one record at a time, as conventions.md § *A decision record* says (`docs/agent/conventions.md:495`). The brief also tells each subagent to leave plain-text record lists alone (`docs/agent/doc-cleanup.md:46`, `docs/reference/findings.md:18`, this file), to report and not change a user-facing diagnostic naming a record, to rename no test, and to run no cargo:

- [ ] **The middle ones, two to a subagent** — 0132 (11 citing files), 0148 (10), 0142 (9), 0139, 0150 (7), 0124, 0146, 0153 (6). `docs/decisions/0132.md:1` `docs/decisions/0148.md:1`
- [ ] **The narrow ones, three to a subagent** — 0122, 0125, 0126, 0127, 0129, 0130, 0131, 0133, 0134, 0135, 0136, 0137, 0138, 0143, 0144, 0145, 0147, 0149, 0151, 0152, 0154, 0155, 0156, 0157, 0158, 0159 (1 to 5 citing files). `docs/decisions/0122.md:1`
- [ ] **Close the stage** when `git ls-files 'docs/decisions/01[2-5]*'` is empty; its status is `docs/goals/decision-records-retired.md:49`.

### Backlog

- Stages 7 to 10: merge every record by number range, rewrite each one's citations in the same slice, and delete it.
- Left by the Stage 4 merges, unchecked: `rule:classes/an-unwritten-property-read-throws` says only `lateinit` reaches the never-written state, but `crates/nvs-types/src/ctor_init.rs` says an unassigned hooked property does too; the shipped `crates/nvs-config/src/default.toml` sets no `[limits.hard]` ceiling, so per-request memory has no cap by default; the playbook bullet `docs/agent/playbook/tooling/a-loop-goal-toml-exact-check-over-an-example-now-pins-the.md` names a check that no longer exists; `examples/cli.nvs`, `examples/crypto.nvs:9` and parts of `examples/isolate.nvs` still fail `bun nv proofs --comments`.
- `bun nv verify` failed once at conformance with every case green: `tools/nv/select/record.ts:453` found no recording for 11 cases that the same `recorded` map held at `:477`. It failed the same way for 31 cases in a later session. Each time a rerun was green, and the cause is not found.
- Rules the Stage 5 merges found disagreeing with the code, left unchanged: `rule:types/enum-case-type` and `rule:types/narrowing` say `$m == Mode::Read` does not narrow, but `tests/conformance/lang/an-enum-case-comparison-narrows-its-subject.nvst` shows it does; `rule:config/scope-has-no-default` says a `fleet` entry with no shared store stops the boot, where `rule:config/a-fleet-entry-fires-at-most-once-under-a-lease` says it is left unarmed with a note. A test name still carries a record number: `crates/nvs-types/tests/visibility.rs:233`. `crates/nvs-types/src/testing.rs:699` is a diagnostic help string naming "§ 9" of a merged record; it means `rule:testing/data-rows`. `Cargo.toml:540` says `raw_value` writes a `decimal` as an exact JSON number, but `crates/nvs-stdlib/src/json.rs:143` writes a string. `rule:config/the-extension-set-is-in-every-unit-key` gives `content_hash` as the source alone, where `rule:packaging/an-artifact-is-one-immutable-content-addressed-file` says name, text and folder.
- Found by the Stage 6 merges, left unchanged: the three `expressions/pipeline-*` rules and five `ide/redaction-*`/`ide/tainted-*`/`ide/phpstorm-*` rules are `designed` though the code ships them (`crates/nvs-syntax/src/parser/expr.rs:661`, `crates/nvs-lsp/src/redactions.rs`), and those `ide` fragments still say there is no language server; `rule:errors/multipart-part-count` names a `max_multipart_parts` directive, but the cap is a constant (`crates/nvs-stdlib/src/multipart.rs:55`); `crates/nvs-lsp/src/completion.rs:9` claims PHP built-in completion that no code does; `crates/nvs-cli/src/service.rs:3941` names a virtual account where the default is `LocalSystem`; `crates/nvs-cli/src/service.rs:4277`, `:6231` and `:6464` are test messages naming a merged record's `§ N`; `crates/nvs-diagnostics/src/lib.rs:5817` is a test name carrying a record number; no fuzz target covers the HTTP message parser. Also unchecked: `crates/nvs-config/src/mode.rs:25` says the mode defaults apply only on a runtime flip, though `rule:config/a-mode-is-five-defaults` is `shipped`; `crates/nvs-host/src/stack.rs:54-57` says the in-flight cap does not exist, though `crates/nvs-server/src/admit.rs` admits; no consumer warns on registry/spec drift as `rule:core-api/field-wise-precedence` claims; no test pins the lowercased-verb `operationId` suffix (`crates/nvs-cli/src/openapi.rs:96`); hover renders `@see` as prose (`crates/nvs-lsp/src/hover.rs:76`); the test `the_mode_table_is_exactly_the_five_directives_the_adr_lists` (`crates/nvs-config/tests/request.rs`) names "the ADR"; `website/src/data/core.json` lists the merged `"0018"` under `Core\Debug`.
- Bare `§ N` marks of merged records are left wherever the module head never named the record, so the merges could not map them: `crates/nvs-cli/src/cache.rs` (0042, about 100), the `crates/nvs-db/src/` drivers and `crates/nvs-stdlib/src/db/` (0067), `crates/nvs-server/src/` (`serve.rs`, `schedule.rs`, `mount.rs`, `forwarded.rs`, hundreds, mostly 0074, 0095, 0097, 0106), `crates/nvs-config/src/{mount,server,tree}.rs`, the queue modules (0084: `crates/nvs-stdlib/src/queue.rs` about 160, its tests, `crates/nvs-cli/src/{worker,serve}.rs`, `crates/nvs-db/src/plan.rs`), `crates/nvs-host/src/isolate.rs` (0083), `crates/nvs-stdlib/src/log.rs`, `crates/nvs-render/src/json.rs` and `crates/nvs-runtime/src/floor.rs` (0020, 0076), and `crates/nvs-stdlib/src/http/transport.rs:18`. Nothing checks them; Stage 13's guard decides whether it should.
- `worker::tests::a_served_process_with_workers_exits_within_its_deadline_once_the_drain_begins` (`crates/nvs-cli/src/worker.rs:3056`) failed once under a full `bun nv verify` at 2.5 s and passed alone and on the rerun.
- `website/src/data/core.json` still lists merged record numbers in its `adrs` fields until `bun nv render --website` runs; its unrelated drift was reverted each time. `rule:core-classes/pdf-one-engine` is rewritten by the `pdf` goal's Stage 2.
- Gaps the merges found and left unfiled (`data/gaps/`): `else if` is not normalized to `elseif` (filed, `data/gaps/nvs-fmt/else-if-is-kept-as-two-words.json`); `docs/rules/ide/redaction-ranges-come-from-the-server.md:22` says there is no language server; no test calls `$obj(...)` for the no-`__invoke` diagnostic (`crates/nvs-types/src/expr/calls.rs:1372`); `rule:testing/perf-two-mechanisms` names a callgrind runner CI does not have; no dead-letter depth series exists though `examples/queue.nvs` once claimed one; no per-request cap on `{shared: false}` database connections (`crates/nvs-stdlib/src/db/`); a second throw in a task group goes to stderr, not `Core\Log` (`crates/nvs-host/src/group.rs:76`); `#[TestSource]` and `assertStartsWith` do not exist; no test resolves a Windows absolute `script.spawn` root.
- Stage 11: merge and delete the rest of `docs/adr/`.
- Stage 12: the rest of the tools forget records (`tools/nv/cmd/pack.ts:47` `SHAPE_IMPLIES`, `pack.ts:221`).
- Stage 13: the process docs (`docs/agent/doc-cleanup.md:129`, `docs/agent/loop-authoring.md:125`, the playbook bullet `tooling/a-decision-records-changes-block-needs-modifies-spelled-out`) and the guard; `nv rules --check` must be green first.
