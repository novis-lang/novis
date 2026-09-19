A feature is finished when its **feature proofs** exist, not when it works. Each one proves a
different thing about the feature, and this list is the only place that says which they are.

**Description** — a short page of plain prose saying what the feature does, which a beginner and an
expert read the same way. It is the first thing the website shows for the feature, it is written
first, and every kind of feature owes it.
**Tests** — its behaviour pinned from Novis *and* from Rust; a `Core` member owes at least one of
each. **Examples** — three small, self-contained, plainly-commented programs a reader learns from.
**Perf** — one measured figure, so a change can be re-measured against it. **Hostile** — one program
written to break it, which passes when the runtime is still standing.

Each tree's own README owns what a file in it *is*, and this rule restates none of them.

Not every kind of feature owes every proof: an enum is not attacked and a directive is not
benchmarked.
What each kind owes is **data**, overridable per feature, because a policy stated only in prose is a
policy nothing can check. A single feature excused from a single proof is a skip entry carrying
**the reason as its value**, so "this cannot be measured" and "nobody wrote one" never look the same
in the audit.
