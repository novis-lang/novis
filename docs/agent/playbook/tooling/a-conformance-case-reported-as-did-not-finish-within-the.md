- **A conformance case reported as `did not finish within` the runner's deadline is a cost failure, not
  a wrong answer, and the cases that reach it spawn child processes.** The sweep runs the suite pooled
  beside cargo builds, so every process creation and pipe wakeup becomes a scheduling question, and a
  child dribbling its output a line at a time costs orders of magnitude more to drain than the same
  bytes in bulk. Reproduce under `nproc`-many busy loops before reading the tree as regressed, and see
  `nvs_test::run::CASE_TIMEOUT`'s doc for what the deadline is for. [until: gone crates/nvs-test/src/run.rs:did not finish within]
