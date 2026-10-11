A feature is finished when its **feature proofs** exist, not when it works. Each one proves a
different thing about the feature, and this list is the only place that says which they are.

**Description** — a short page of plain prose saying what the feature does, which a beginner and an
expert read the same way. It is the first thing the website shows for the feature, it is written
first, and every kind of feature owes it.
**Tests** — its behaviour pinned from Novis *and* from Rust; a `Core` member owes at least one of
each. **Examples** — three small, self-contained, plainly-commented programs a reader learns from.
**Perf** — one measured figure *and its growth*: the figure, so a change can be re-measured against
it, and the doubling ramp of `bun nv scaling` over the same bench, so a cost that grows faster than
the bench's declared `// bench: complexity` fails before it ships. The counts decide and the clock
reports; valgrind runs only when the ramp shows no clear growth, only to find the batch at which it
is clear, and never in `nv verify` or an ordinary bench run. **Hostile** — one program written to
break it, which passes when the runtime is still standing.
**Help** — the binary that runs the feature explains it. `nvs agent find` reaches the feature under
the name its user types, and `nvs agent show` prints what it does, how it is written and what it
throws or reports: a `Core` member's card, a language feature's reference section, a command's or a
configuration key's own entry. It is written with the feature and changed with it, in the voice of
AGENTS.md § *Text an end user reads*, because a user and their coding agent look there before
anywhere else, and a feature the binary cannot show them reads as one that does not exist. The
features that landed before Help was a proof are listed in `data/proofs/help-backlog.json` and read as
skips with one shared reason, so a finished goal's floor does not fail over a proof that was not owed
when it finished; a feature leaves the list when it gains its help.

Each tree's own README owns what a file in it *is*, and this rule restates none of them.

Not every kind of feature owes every proof: an enum is not attacked and a directive is not
benchmarked.
What each kind owes is **data**, overridable per feature, because a policy stated only in prose is a
policy nothing can check. A single feature excused from a single proof is a skip entry carrying
**the reason as its value**, so "this cannot be measured" and "nobody wrote one" never look the same
in the audit.

The proofs are a cost spent up front, per feature, and spent on security and correctness: an attack
per feature is a security question asked before a user asks it. What keeps the audit affordable to
re-run is that its gate runs no program. It checks what exists on disk, so a sweep over a finished
tree is a walk of the directories. An audit that reads mostly red is the honest number, never a
reason to loosen what a kind owes; turning one proof off for the whole roster changes what is owed
and says so in the audit.

**Declined:**
- **Line coverage instead of a test from Rust.** A coverage run says which lines ran, not which
  feature is proven, and a coverage percentage is a threshold rather than a finished feature.
- **A hand-kept checklist of finished features.** It rots within a day of an unattended run
  (`rule:testing/roster-is-derived`).
