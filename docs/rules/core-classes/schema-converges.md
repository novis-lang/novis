A schema value says what the tables **should be**. A plan is the difference between that and what a
connection **has**. That is the entire model, and everything a versioned migrator carries is absent
by construction: no version number, no history table in the target database, no ordering between
changes, no `up`/`down` pair, no fleet lock, and no notion of which migrations have run.

Convergence is what makes that possible. A plan is derived from the current state every time it is
computed, so it is correct after a manual change, after a partially applied earlier plan, and against
a database this tool has never seen — the three situations in which a history table is exactly wrong,
because it records what a tool believes rather than what is true.

**`Core` therefore holds no notion of a migration.** Ordering, reversibility, dependency between
changes and fleet locking stay a blocked gap in the first-party framework
(`rule:programs/no-migration-runner`), and this is the layer such a runner would one day sit on. It
stops there deliberately: every one of those open questions is a question convergence does not have
to answer.
