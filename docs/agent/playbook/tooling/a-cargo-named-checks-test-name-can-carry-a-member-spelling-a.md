- **A `cargo-named` check's *test name* can carry a member spelling a later ADR renamed, and
  writing that member is the wrong repair.** Goal `net-os-signal` asked for
  `..._memory_usage_..._all_answer` after ADR 0148 § 12 had moved held bytes to `Core\Budget`,
  leaving `Core\Os::residentBytes`. A check name is drafted before its stage runs, so when the
  spec row, the migration table and the outstanding file all disagree with it, amend the name.
  [until: reviewed 2026-09-09]
