When two slices compete for a session, the one that gets a working web application sooner wins. This
binds the plan, in this order:

- **The framework and the dependency story are milestones, not conveniences** — they are the two
  things a new user hits before reaching a language feature. They do **not** outrank language and
  library completeness, because a user arrives at a language that runs their program, and a package
  registry in front of a runtime that cannot serve a request or talk to a database ships an excellent
  way to install nothing. This bullet binds wherever the competing slice is `Core` **breadth** —
  another twenty members on a class that already works — and never against the capability that makes a
  whole domain reachable for the first time.
- **A security property already decided but not yet enforced outranks new `Core` breadth.** The
  qualifiers are the product, and a `Core` member that does not carry them correctly is worse than a
  missing one.
- **The multi-tenant shape wins ties**, as an engineering default rather than an audience one: where a
  design could serve one application or many on a fleet at the same cost, it serves many — per-tenant
  budgets, limits, observability labels, and a connection reset that is a security boundary rather
  than an optimisation. The single-application case loses nothing by it, which is what makes this a
  tie-break rather than a priority.
