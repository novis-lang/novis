- **`python tools/dossier.py --record-perf --group 'Core\Cli'` does not reach `Core\Cli\Color` or
  `Core\Cli\Live`.** A group is matched as the class name itself, not as a prefix, so a goal whose
  features live in nested classes leaves those benches reading `perf stale` after what looks like a
  whole-group run. Record each class with its own `--group`, and read the `N features` line the run
  prints against the number of features the goal owes. [until: reviewed 2026-09-21]
