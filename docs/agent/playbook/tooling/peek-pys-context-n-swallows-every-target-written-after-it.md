- **`peek.py`'s `--context N` swallows every target written after it.** `python tools/peek.py
  A.rs:@sym --context 20 B.rs:@other` stops with `unrecognized arguments: B.rs:@other`, because the
  option's value and the positional targets are one argparse list and the run of targets after the
  flag has nowhere to land. Put `--context`/`--window` after the last target, or use the target's own
  `:3` suffix, which wins over the flag anyway. [until: reviewed 2026-09-16]
