- **A dossier acceptance check whose detail reads `target/release/nvs.exe is missing or older than
  the tree` failed on something else: that line is only the rebuild notice.** `current_binary`
  prints it to stderr, rebuilds release itself and goes on, and the driver's detail shows the first
  line of output, so the real verdict — here `perf: stale: crates/nvs-stdlib/src/io.rs changed
  since it was last measured` for every member of a class sharing the edited file — is hidden
  behind it. Re-run the check's own argv by hand and read its tail; any edit to a member's
  implementing file (a class card for a sibling class included) makes every figure measured from
  that file stale, so run `--record-perf` for each group in the file after the last edit.
  [until: reviewed 2026-10-24]
