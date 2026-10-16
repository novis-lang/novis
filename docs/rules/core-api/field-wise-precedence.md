For an implemented member, a documentation field the registry carries is authoritative and a field it lacks
falls back to the spec-derived default. For a member not yet implemented, the spec is all there is. A
consumer that finds the two disagreeing about something both state — a parameter documented in one and not
declared in the other, a signature that has drifted — warns rather than silently choosing.

The spec stays authoritative for the **surface**: every member, implemented or not, its signature as
designed, its types and the reasoning behind its design. What it stops being, for an implemented member, is
the place the card's wording is edited (`rule:core-api/reference-card`).

Precedence is **field by field** rather than source by source, and that is what made the move incremental:
the seam shipped empty, every member documented afterwards upgraded on its own, and no consumer was ever
blocked on an unfilled field. It is also what keeps a consumer correct against a toolchain older than the
backfill.

The warnings are the drift detector. Nothing else notices that a member's spec entry and its implementation
have diverged, because the two are read by different tools for different reasons.
