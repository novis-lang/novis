- **The acceptance failure the pack quotes can already be repaired on disk, because commits land
  between the sweep that wrote the line and the session that reads it.** Goal `m5-proofs`' 100k-task
  check was reported "did not run" by a sweep that ended `17:06`, and the rename giving that test
  the name the check filters on was committed at `18:50:52`, before the run the pack came from
  began. Compare the ledger entry's run header against `git log --date=iso` before diagnosing, and
  where head is the newer of the two, re-run the check's own `argv` and believe that.
  [until: reviewed 2026-09-14]
