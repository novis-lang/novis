- **A goal's absence gate greps a *path list*, and a path left off it is where the retired spelling
  survives.** Goal `one-type-test`'s gate named `crates`, `tests` and the doc trees but not
  `examples`, so it stayed green while three example programs still spelled the type test PHP's way and
  the floor went red on one of them. Spelling the word inside the bullet is the second way to redden
  such a gate, since the playbook is on its path list. When a gate's claim is "this word appears
  nowhere", run `git
  grep -l -i -w <word>` over the whole tree and compare it against the check's `argv` before treating
  the gate as the specification. [until: reviewed 2026-09-18]
