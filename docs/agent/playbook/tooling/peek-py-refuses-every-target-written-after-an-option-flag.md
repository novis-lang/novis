- **`peek.py` refuses every target written after an option flag.** `peek.py a.rs:re:x --context 30
  b.rs:re:y` dies with `unrecognized arguments: b.rs:re:y`, which reads like a misspelled target and
  sends you hunting the locator syntax instead of the argument order — argparse stops collecting
  positionals at the first optional. Put every target first and the options last, or give each target
  its own `:N` context suffix, which wins over `--context` anyway. [until: reviewed 2026-09-15]
