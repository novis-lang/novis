- **A matrix case's queue name isolates its rows but not its locks, and the failure lands in a
  neighbouring case.** InnoDB locks what a statement scans, so `clear`'s `delete … where queue = ?`
  holds every scanned row until commit, and a neighbour's `for update skip locked` skips its own
  row. `framed()` takes `FRAMED_WRITES`; when a `db-matrix` leg fails in a case you did not touch,
  rerun it before believing it. [until: gone crates/nvs-stdlib/tests/queue.rs:FRAMED_WRITES]
