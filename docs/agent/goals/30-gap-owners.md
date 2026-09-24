---
milestone: post-parity
---
# Loop goal 30 — a module doc's gap names its owner, and a gate says so

Every crate module doc records what its subsystem still owes under `# Known gaps`. There are **50 such
blocks holding 152 enumerated items**, and `carried-gaps.md` indexes 22 of them
while [carried-refusals.md](../carried-refusals.md) covers `nvs-ir`'s 15. The rest — about 110 recorded
gaps — name no owner, appear in no index, and are invisible to every tool, because nothing in the tree
is shaped wrong. When this goal is green each one carries an owner in its own doc, a tool derives the
list rather than copying it, and a gap added without an owner fails a check.

**This is the same failure `carried-gaps.md` was created to fix, one level down.** That file exists
because the handoff could not hold a gap; it turns out the module docs could not either, because
nothing read them. Goal `carried-gaps` stage 2 built the gate for an *outstanding spec key* — a key whose owner is
not a live goal fails. This goal builds the same gate for a *module-doc gap*, and the argument is
goal `carried-gaps`'s argument verbatim.

## Why here

**It sits after the M8 goals** because those close a large share of what would otherwise need
attributing, and attributing a gap that is about to be closed is work done twice. Goal
`carried-gaps` stage 2 built the gate for an outstanding spec key — a key whose owner is not a live
goal fails — and this is the same gate for a module-doc gap.

## Stage 0 — the catch-up

1. **`carried-gaps.md`'s two sections keep their shape.** Goal `carried-gaps` built a check against them and this
   goal does not move the file out from under it. What changes is that the file stops being the *only*
   record: it keeps the entries whose ownership needs an argument, and the machine list is derived.
2. **The four attribution errors already known** are corrected as part of this stage rather than left
   for the tool to report: `Core\Metrics` is goal `server`'s and was listed unowned; § 17's four are M8's
   Tier 0 roster and `spec-classes-part-two-outstanding.txt` said M9 carried them. Goals `per-core` through `formats` and 29
   struck
   most of this; whatever survives is corrected here.

## Stage 1 — the floor

Goal `xml-tree`'s whole acceptance list, carried in verbatim by `tools/goal-switch.py`. Never traded.

## Stage 2 — the keystone: an owner tag, in the doc that owns the gap

1. **A gap item ends with an owner tag.** One trailing marker per enumerated item in a `# Known gaps`
   block — `— owner: unowned-sweep`, `— owner: M9`, or `— owner: unowned`, on a line of its own at
   the item's end. The tag sits with the gap because
   every fact in this repository has one home and a gap's home is its module; the index is then
   *derived* rather than a second copy, which is the rule `holes.py` already follows.
2. **Three owner kinds and no fourth.**
   - **A goal slug** naming a live goal in [the goals directory](README.md) — never its
     number, which is a position and moves the moment anything is inserted ahead of it. The entry
     closes the gap or the tag is wrong.
   - **A milestone tag** — `M9`, `M11` — for a gap a *future* milestone's plan already covers. This is
     not an unclosed gap; it is scheduled work, and conflating the two is what made the list of 110
     look alarming. The milestone's own file must state the scope, and the tool checks that the
     milestone exists and carries no live goal that should have claimed it instead.
   - **`unowned`**, which requires a bullet carrying the reason: in `carried-gaps.md` § *Unowned*, or,
     for a gap whose sites `carried-refusals.md` already carries, in the entry that holds them. This is
     a legitimate state and a scheduling question for the user; it is never the absence of an answer.
3. **`tools/owners.py`, derived and never copied.** It walks every `//! # Known gaps` block in
   `crates/*/src/**`, extracts each enumerated item and its tag, and prints the roster: owned by goal,
   deferred to a milestone, unowned, and — the interesting output — **untagged**. `--unowned` prints
   only the scheduling questions; `--json` prints one object. Its help text carries the argument for
   the three kinds, the way `holes.py`'s and `gaps.py`'s already do.
4. **The gate.** An untagged gap item fails, and so does a tag naming a goal that is not on the chain,
   a milestone that does not exist, or `unowned` with no bullet behind it. This is not an exemption
   list; there is no allowlist and adding one is the move the gate forbids.

## Stage 3 — the attribution pass

1. **Every one of the ~110 items gets a tag**, module by module, and the judgement is the work: a gap
   that a future milestone plans takes that milestone, a gap a live goal's item list covers takes the
   goal, and everything else is `unowned` with a reason written into `carried-gaps.md`.
2. **Known deferrals, so a session does not re-derive them**: `crates/nvs-cli/src/bundle.rs`'s `.nvsx`
   embedding is M9's; the inlining items in `crates/nvs-runtime/src/decimal.rs` and the string
   fast-path in `crates/nvs-runtime/src/lib.rs` are M12's optimising tier.
3. **Known non-gaps, struck rather than tagged**: an item that says it is deliberate is not a gap —
   `crates/nvs-stdlib/src/time.rs`'s "there is no `Core\Month`, and there is not going to be one" and
   `crates/nvs-syntax/src/casing.rs`'s "left out deliberately" are decisions, and they move out of
   `# Known gaps` into the module doc's ordinary prose so the roster counts what is owed.
4. **A stale gap is closed, not tagged.** `crates/nvs-stdlib/src/router.rs`'s gap 3 says
   `Core\Router::match` is absent while the plan's *Open now* says a request is matched — one of the
   two is wrong, and finding out which is this stage's job.
5. **`carried-gaps.md` § *Unowned* is rewritten from what the pass finds**, and it is the only place a
   reason lives.

## Stage 4 — it stays true

1. **`nv verify` runs the gate**, so a gap added without an owner fails before it is committed rather
   than two milestones later.
2. **`brief.py --where` routes to it**, and the orientation pack names the count of unowned items so a
   session sees the number without reading the file.
3. **The tag survives a goal switch.** `tools/goal-switch.py` carries checks forward and item lists not
   at all — which is the mechanism that orphaned everything this goal indexes — and a tag in a module
   doc is untouched by it. That property is what makes this the right home and it is stated in the
   tool's own doc.

## Standing decisions

- **This goal opens no ADR number.** It is a process gate over documents that already exist; nothing
  about the language, the runtime or an interface changes.
- **The owner lives with the gap, never in a second list.** An index that is maintained by hand
  alongside the thing it indexes is the failure this goal exists to end, and building a second one
  would repeat it.
- **A future milestone's plan is an owner.** Only a gap in a milestone that has already been carried —
  its goals walked or closed — and that no chain entry claims is `unowned`. This distinction is the
  whole point: without it the roster is 110 alarming items, and with it it is a short list of real
  scheduling questions.
- **A `carried-refusals.md` entry is a reason, not an owner.** It names refusal *sites*, and a site is
  not a goal, so the three kinds stay three: a gap block those sites index is `unowned`, and that entry
  is its reason. `owners.py`'s `unowned_paths` reads that file beside `carried-gaps.md` for the reason
  it reads either — the entry already names the module doc that owns the detail, so the link is the
  path and no key has to be invented. The alternative, reading an owner off the entry's prose sentence
  naming the goal that closes it, is the failure this goal exists to end, and it has already happened
  once: entry 901 named goal `typed-callable`, that goal retired without closing a site, and all
  fifteen are still open.
- **`unowned` always carries a reason, and the reason names what has to be decided.** "Nobody has got
  to it" is not a reason; "this needs an options bag that can tell an omitted option from a written
  `null`, which is a registry question" is.
- **The gate has no allowlist.** A gap that cannot be tagged is a gap whose owner has to be decided,
  and that decision is cheap exactly once — when the gap is written.
- **Ambiguity about whether something is a gap or a decision resolves toward *decision*, and it moves
  out of the block.** A `# Known gaps` list that holds settled non-goals is a list nobody trusts.
- **What this spends**, per `rule:programs/memory-priority`: nothing at run time.
  One tool invocation in `nv verify`, over doc comments already parsed by nothing.
