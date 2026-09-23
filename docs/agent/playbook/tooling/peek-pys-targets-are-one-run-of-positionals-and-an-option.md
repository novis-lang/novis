- **`peek.py`'s targets are one run of positionals, and an option written between two of them drops
  everything after it.** `python tools/peek.py A.rs:1-9 --context 4 B.md` answers `unrecognized
  arguments: B.md`, because `argparse` closes the positional list at the first option it meets and
  the tool's usage line does not say so. Write every flag before the first target — `peek.py
  --context 4 A.rs:1-9 B.md` — or drop the flag and give the one target that needs it its own
  `:re:pat:3` suffix, which is per-target anyway.
  [until: gone tools/peek.py]
