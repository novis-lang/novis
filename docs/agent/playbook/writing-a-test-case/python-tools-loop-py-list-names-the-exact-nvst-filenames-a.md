- **`python tools/loop.py --list` names the exact `.nvst` *filenames* a goal owes, and a case under
  another name does not count.** The owed name is also a *specification* — each clause of it is a
  row the case must have. Run `python tools/holes.py --item N` before writing, which prints the same
  names under "cases that may belong to it"; renaming afterwards is a `git mv` plus a re-run.
  [until: reviewed 2026-09-06]
