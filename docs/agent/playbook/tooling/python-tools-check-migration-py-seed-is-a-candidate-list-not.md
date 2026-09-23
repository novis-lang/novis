- **`python tools/check-migration.py --seed` is a candidate list, not rows to paste.** It attributes
  every backticked PHP name on a line to whatever that line is *about*, so a member's own prose
  drags its neighbours in (`acos` → `Core\Math::asin`), and a right member name against the wrong
  PHP function passes the checker silently. Fill a domain the other way round: read
  `01-core-library.md`'s table for the class once, then write one row per name `--report` still
  lists. [until: gone tools/check-migration.py:--seed]
