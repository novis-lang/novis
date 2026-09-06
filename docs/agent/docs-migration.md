<!-- rules-py:examples — the rule ids below are illustrations, not citations, and
     `python tools/rules.py` skips this file's `rule:` tokens because of this marker. -->

# The docs migration — rules, decisions, and the end of the fold

A one-time restructuring of how this repository records what it has decided. It runs over many
sessions, it is driven by `python tools/migrate-docs.py`, and **this file is the contract**: the
driver's work orders cite it, and a session that needs to know why a step exists reads it here.

Delete this file, and `tools/migrate-docs.py` with it, when unit C8 lands.

## Why

Every settled decision lives in one ADR, and that ADR is three documents wearing one jacket:

| Job | Lifecycle | Read by | What sharing a file costs |
|---|---|---|---|
| **the rule** — what is true now | edited forever | anyone applying it | forces the *fold*, which is the sole origin of `Amends:`, `Amended by:` and 4,507 internal cross-links |
| **the rationale** — why, and what lost | written once | someone about to overturn it | 646 KB carried inside every read of a rule |
| **the history** — what changed when | append-only | nobody; `git log` has it | the amendment metadata |

Two consequences follow, and they are the whole reason for this migration.

**The rule is filed per decision rather than per topic.** So one topic's rules shatter across many
ADRs and get stitched back together with links. ADR 0007 § 2's conversion table is well written and
correctly folded, and it still hands the reader off to 0054, 0009, 0125, 0144, 0126, 0024, 0033,
0035, 0034, 0066 and 0028 to complete one table. A person cannot hold that. An agent barely can.

**Growth runs on the wrong axis.** Files grow with decisions taken; a language's rule surface grows
with topics, which is roughly fixed. 143 ADRs describe about 22 topics. Four hundred ADRs will still
describe about 22 topics — and there will be four hundred bodies to fold.

The fix is one sentence: **code cites the rule, not the decision.** Today a doc comment says
"ADR 0010 § 3's …" when it means "the rule that an enum constant is …". Naming the decision is what
obliges the decision record to stay eternally current. Name the rule instead, and the record is free
to freeze the day it is accepted.

## The end state

```
docs/
  rules/                      NORMATIVE. always currently true. bounded by the language.
    _index.json                 topic order, the rule-id registry, ownership boundaries
    types.json                  STRUCTURE ONLY — ids, status, because, flags, order. no prose.
    types/                      PROSE FRAGMENTS — one .md per rule. no metadata. no escaping.
      conversion.md
    types.md                    GENERATED — the read surface, for humans and for agents

  decisions/                  IMMUTABLE RECORD. frozen on acceptance. full text, untrimmed.
    0002-error-propagation.md   frontmatter: date, status, changes = [rule ids]

  agent/                      process knowledge — bounded, current-only, expiry-checked
  plan/                       milestones — unchanged by this migration

  novis.md        GENERATED   the shipped-only manual (filtered on a rule's `status`)
  decisions.md    GENERATED   plain language, for the website
  divergences.md  GENERATED   every rule carrying `divergesFromPhp`
  ground-rules.md GENERATED   one line per rule
```

Two authored trees. Everything else falls out of them.

### The rule id is a path

A rule id **is** its fragment's path: `types/conversion` is `docs/rules/types/conversion.md`. ASCII,
greppable, and semantic in a way `0007 § 2` never was. Because an id is namespaced by its topic, two
topics cannot collide on one — which is what makes parallel authoring safe rather than hopeful.

The canonical citation token, everywhere — Rust doc comments, `.nvst` cases, goal manifests, one rule
citing another — is `rule:types/conversion`. `grep -rn 'rule:' ` finds every citation in the
repository, which is what makes the gate's first check cheap. Generated markdown linkifies the token;
source files keep it flat.

### Why JSON plus fragments, and not one file

JSON has no multi-line string, so a rule body stored *in* JSON would be one line with every quote and
newline escaped — unreadable, and a one-word edit would diff as a whole line. TOML's `'''` literal
strings have the opposite problem from the one usually assumed: they are the least escape-prone form
available, which is why `docs/decisions.toml` uses them.

The split avoids the question. JSON carries **structure only** and therefore contains no prose to
escape; the prose is real markdown in its own file. Agents manipulate clean JSON, humans edit real
markdown, and neither format is asked to do the other's job.

```jsonc
// docs/rules/types.json
{
  "topic": "types",
  "title": "Types",
  "order": 20,
  "rules": [
    {
      "id": "types/conversion",
      "title": "Conversion",
      "status": "shipped",
      "because": ["0007", "0066", "0054", "0009"],
      "divergesFromPhp": "(int)$x never parses; `as` is the only conversion spelling",
      "seeAlso": ["types/qualifiers", "errors/throwing"],
      "guardedBy": ["tests/nvst/types/conversion.nvst"]
    }
  ]
}
```

`status` is `shipped` or `designed`. `reference.py` filters `novis.md` to `shipped`, so that file
keeps the property that every line in it runs, while the rulebook carries the whole designed surface
and `docs/spec/` stops needing to exist.

### What a decision record becomes

Frozen on acceptance. No `Amends:`, no `Amended by:`, no fold, ever again. It gains one machine field
and loses all of its maintenance:

```markdown
---
date: 2026-08-23
status: accepted            # accepted | superseded-by 0102
changes:
  creates:  [types/nullable-conversion]
  modifies: [types/conversion, core-api/no-try-prefix]
---
# 0066 — `expr as ?T` converts without throwing

## The question    ## The options    ## Why this one    ## What it costs
```

Records stay in-repo at full length. Their 646 KB of rationale stops costing anything the moment
nothing reads them to find a rule, and that rationale is exactly what is wanted when a decision is
revisited — so the trim pass `doc-cleanup.md` § 4 has been deferring is **cancelled, not deferred**.

The reverse index — which decisions shaped `types/conversion` — is derived from `changes:`. Nothing
about the relationship between a rule and a decision is ever maintained by hand again. That is the
`Amended by:` failure taken out by construction.

## The rule this migration never breaks

**A subagent never writes into the repository tree.** It reads, and it writes an *apply-file* into
the scratchpad. Only `python tools/migrate-docs.py --apply` touches `docs/`, `crates/` or the goal
manifests — in the driving session, one transaction at a time, each through the full gate.

That single rule dissolves subagent-versus-subagent conflicts, subagent-versus-loop conflicts, and
any possibility of a half-applied topic, all at once.

## The gate

Every transaction passes all six or restores every byte it touched.

```
1. no citation anywhere resolves to nothing — crates/, docs/, tests/, tools/, goals
2. every goal manifest's rules/adrs/spec entry resolves to a live rule id
3. the orient.py pack for all 20 goals, diffed against the A3 snapshot:
   NO GOAL MAY LOSE INFORMATION. added context passes; dropped context fails.
4. reference.py regenerates novis.md and every example still runs green against the binary
5. check-links.py, plan.py --check and rules.py --check all clean
6. python tools/verify.py green
```

Check 3 is the whole basis of the guarantee that planned goals keep working, and it is why the
snapshot is unit A3 rather than an afterthought. **Nothing moves before it exists.**

### The legacy debt check 1 exempts

Taking the snapshot found **16 citations that were already broken** before the migration began —
a `§ 1` of record 0002 from `nvs-cli/src/cache.rs`, where that record has no numbered sections at all;
`0140` and `0141` from two goal handoffs, where neither record exists; and thirteen more. Unit C8
re-cited the ten that were left to the rules that hold them.
`python tools/adr.py` reports clean because its section-ref check does not reach into `crates/`.

The gate cannot demand zero from unit B1 — it would fail every transaction for debt that predates
the migration — so check 1 gates on **new** breakage against `.migration/snapshot/known-dangling.json`
and lets the recorded set through. **`--sweep` grants no such exemption**: at C8 the legacy set has
to be gone, which is where those sixteen get fixed.

## The units

Thirty-six, each sized to one session. `python tools/migrate-docs.py --status` is the live state;
the table below is the shape and the order.

### Phase A — build the machine, move nothing

| Unit | Deliverable |
|---|---|
| A1 | `tools/rules.py` — load and validate JSON plus fragments, resolve ids, detect orphans, cycles and duplicates, render `docs/rules/*.md` and every generated artifact |
| A2 | `tools/migrate-docs.py` — the resumable driver: state file, unit registry, `--status`, `--next`, `--apply`, `--gate` |
| A3 | **The snapshot.** Every goal's `orient.py` pack, `brief.py`, `novis.md`, and all 586 distinct `(ADR, §)` citations with their resolved target text, captured before anything moves |
| A4 | The gate, all six checks, wired into `verify.py` |
| A5 | **The topic map** — every one of the 960 anchors assigned to exactly one topic. Reviewed by the user before Phase B opens, and `--topic-map` refuses to overwrite it afterwards |

A1 and A3 are disjoint and may run in parallel. A5 is what makes fan-out safe: without it, two
authors independently claim "a successful `as` strips `tainted`" for `types/conversion` and for
`security/launder`, or neither does. It also gives the gate its completeness measure — sections
mapped but unclaimed, which must reach zero at C8.

### Phase B — one topic, one transaction

`B1 errors` is the pilot and runs alone. It is small, it has few citations, and its output becomes
the **style exemplar** inlined into every later author prompt. The migration pauses for review after
it: if the format is wrong, it is wrong once rather than twenty-two times.

Then, in this order — cheap topics first, so the pipeline is proven before it is stressed:

```
programs  statements  enums  iteration  attributes  testing  expressions
types            ← the heavy one: 0007 0009 0034 0035 0047 0054 0066 0114 0125 0126 0144
classes  core-api  core-classes
security  concurrency  routing  config  packaging  observability  http-server
tooling  ide  php-migration
```

Authoring runs four wide. Applying is **strictly serial**: every apply writes `_index.json`, the goal
manifests, and crate files carrying citations from many topics at once — `registry.rs` alone cites
dozens of records across topic boundaries. Auditing a landed topic against its source records is
read-only and runs parallel again.

### Phase C — sweep and retire

| Unit | What |
|---|---|
| C1 | Freeze all 143 records: strip `Amends:`/`Amended by:`, add `changes:`, move to `docs/decisions/` |
| C2 | Retire `ground-rules.md` and `divergences.md` (now generated) and `adr/README.md`'s routing and index tables. `docs/spec/` **stays live**: `01-core-library.md` is read at test time by `crates/nvs-stdlib/tests/spec_registry_coverage.rs` and by `tools/check-migration.py`, and `02-php-migration.md` is the only home of the per-builtin table `tools/reference.py` renders into `docs/novis.md` |
| C3 | Rewrite `AGENTS.md` — the routing table, the five rules, the session workflow |
| C4 | Rewrite `commands.md`, `conventions.md`, `doc-style.md`, `session-prompt.md`, `loop-authoring.md`, `decisions-summary.md`; delete `doc-cleanup.md` § 4 |
| C5 | `adr.py` becomes decision-record tooling: the fold and amend machinery is deleted, the audit is kept |
| C6 | Re-point `orient.py`, `brief.py`, `dossier.py`, `chain.py`, `plan.py`, `session.py`, `check-links.py` at rule ids |
| C7 | **Expiry for append-mostly knowledge** — every `playbook.md` bullet declares what retires it (a test that now passes, a tool that now exists, a path that is gone); tooling retires the mechanical ones and date-flags the rest; expired bullets are deleted, because `git log` holds them. Same for `carried-gaps.md`, `carried-refusals.md`, `guard-name-debt.md` |
| C8 | **The sweep** — `--sweep`. The deep completeness pass, and the check that the migration is *finished* rather than merely stopped. It **repairs what it can automatically**: re-renders stale generated files, re-points every link into a retired tree, and re-cites what the topic map can resolve — the ~70 citations whose sections split across two topics need every landed chapter's anchor → rule table, which `--apply` banks per unit in `.migration/state.json` under `remap` (B1's was recovered from its commit's diff and is blind to any anchor that had no site). It then names precisely what it cannot: an anchor that never became a rule, a citation that still dangles (the legacy sixteen included, with no exemption here), a record without a `changes:` block, a fragment no rule declares. It exits non-zero until that list is empty |
| C9 | **Self-destruct** — `--self-destruct`. The migration's last act is to leave no trace of itself: `tools/migrate-docs.py`, `.migration/` and this file are removed. It refuses to run until C8 is clean. `tools/rules.py` **survives** — it is the permanent rulebook library — and the tool prints the one migration-only command to strip from it (`--unclaimed`, which reads the topic map). `git log` keeps all of it; `docs/rules/` and `docs/decisions/` are untouched |

C1 and C2 touch everything and are serial. C3 through C6 are disjoint files and run parallel once
rule ids are final.

## What a work order carries

`--next` emits a self-contained session prompt. A fresh session needs nothing else:

```
TOPIC types  (unit B9 of 36)
  ABSORB      the records this topic owns, their § anchors, sliced live
  CITATIONS   every site, pre-resolved to file:line and its current token
  NAMESPACE   the rule ids already taken, and those reserved by pending topics
  EXEMPLAR    B1's chapter, verbatim, as the style to match
  GATE        the six checks and the exact commands
  OUTPUT      one apply-file; --apply does the rest atomically
```

## Where this can go wrong

| Risk | What answers it |
|---|---|
| A goal silently loses context | Gate check 3, against the A3 snapshot |
| The format is wrong at scale | The B1 pilot, and the review that follows it |
| The loop edits the tree concurrently | The driver runs beside the loop, never inside it; each transaction stages only its own files and re-runs the gate before committing |
| A rule falls between two topics | The A5 topic map, and `rules.py --check` counting mapped-but-unclaimed sections down to zero |
| **The topic map is regenerated and the review is lost** | `--topic-map` writes the heuristic's answer over whatever is there, and a heuristic map looks exactly like a judged one. It now counts the rows it would not reproduce — 409 of 960 after A5 — and refuses unless `--force`, which keeps the map it replaced as `topic-map.previous.json` |
| Twenty-two chapters drift in style | The B1 exemplar in every author prompt, and the normalising pass at C8 |
