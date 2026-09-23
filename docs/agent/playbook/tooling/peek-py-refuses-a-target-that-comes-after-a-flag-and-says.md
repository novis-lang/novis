- **`peek.py` refuses a target that comes after a flag, and says `unrecognized arguments` as
  though the target were malformed.** `TARGET` is a `nargs='*'` positional, so
  `peek.py a.rs:@sym --window 30 b.rs:120-160` leaves argparse a second run of positionals it
  has nowhere to put, and the usage block it prints reads as a bad locator rather than a bad
  order. Put every target first and every flag — `--window`, `--context`, `--in`, `--locate` —
  last, and reach for the per-target `:3` context suffix when only one target wants it.
  [until: reviewed 2026-09-11]
