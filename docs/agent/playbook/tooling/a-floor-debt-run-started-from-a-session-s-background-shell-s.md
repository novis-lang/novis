- **A `floor-debt` run started from a session's background shell stops at the shell's two-hour limit,
  and the debt job ends with it.** The check's own timeout is four hours, and the harness kills its
  background commands at two. Each atom is recorded when it ends, so nothing paid is lost, but the
  run's last summary line is never printed. Read the turn lines with `looper exec log <job id>` and
  the store's counts with `SelectStore` in `tools/nv/select/store.ts`, not `.looper/checks/floor-debt.log`.
