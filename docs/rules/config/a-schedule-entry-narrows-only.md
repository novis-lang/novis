An entry's optional `limits` table is a sub-cap on the run's budget, and its optional `grants` table
narrows the run's capabilities. Both are **narrowing only**: an entry cannot raise `memory` above the
deployment's `[limits]`, and cannot grant a capability the deployment's `[capabilities]` withheld or
widen a scope it narrowed. A scheduled run cannot widen anything, for the same reason no program can
(`rule:security/no-runtime-grant`) — the root-owned file is the ceiling, and a block inside it is not a
second authority.

The keys parse and are carried on the entry today. **What is not armed is the narrowing itself**:
nothing in the ticker yet builds a run's budget or grant set per isolate from them, so a fire spends
the deployment's `[limits]` and holds the deployment's `[capabilities]` whole. That is inside the rule
— the run holds no more than the deployment does — and short of it, since an entry that asked for
less does not get less.
