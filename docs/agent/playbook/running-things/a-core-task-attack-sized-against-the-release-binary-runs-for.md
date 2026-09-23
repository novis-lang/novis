- **A `Core\Task` attack sized against the release binary runs for minutes under the debug one the
  hostile sweep uses, and the sweep reports that as `still running after Ns -- unbounded`.** A map
  over 100k elements and a `Core\Task::all` nest 50k deep each finished in well under a second when
  sketched, then took 17s and past 120s as a hostile case, because `dossier.py --run hostile` drives
  `target/debug/nvs.exe`. Time a new task-tree attack with `time target/debug/nvs.exe run <file>`
  before declaring its `timeout-ms`, and size the steps so the whole case lands near two seconds —
  the sweep runs 170 of them. [until: reviewed 2026-10-19]
