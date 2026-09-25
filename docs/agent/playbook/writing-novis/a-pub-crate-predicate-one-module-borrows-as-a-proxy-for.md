- **A `pub(crate)` predicate one module borrows as a proxy for another's roster is a coupling no
  signature shows.** `queue.rs`'s tests asked `nvs_stdlib::db::rendering_for(driver).is_some()` as
  "the queue can send over this one", true only while `Core\Db`'s drivers and the queue schema's
  dialects were the same list, so giving SQL Server an encoder failed two queue tests naming the
  queue. Before widening what a predicate answers `Some` for, `grep -rn '<fn>' crates/<crate>/src`
  and read the doc comment on every hit. [until: gone crates/nvs-stdlib/src/db/pool.rs:fn rendering_for]
