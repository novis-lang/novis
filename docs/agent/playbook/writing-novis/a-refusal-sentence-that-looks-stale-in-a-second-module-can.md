- **A refusal sentence that looks stale in a second module can be stale about a different thing;
  widening it to the first's roster is wrong.** `crate::db`'s `driverless` and `crate::queue`'s gate
  both said "only PostgreSQL runs a statement so far", but db's is about which drivers can send and
  the queue's about which dialect its own SQL is written in. Ask what the module would do with the
  widened case — "the same statements" means a copy, "statements it has not written" means two facts
  — and grep the sentence, not the roster. [until: reviewed 2026-09-06]
