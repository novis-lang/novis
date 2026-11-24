- **A `Core` member declared on two classes owes the conformance floor twice, and the wrong receiver
  counts for neither.** The floor is per class; `corpus::Attribution` attributes `->member(` to the
  classes a case mentions, so three cases writing `$db->queryEach(` leave `Core\Db\Transaction::queryEach`
  at zero. Add a `$db->runTransaction(fn (Core\Db\Transaction $tx) => …)` half, and run `cargo test -p
  nvs-stdlib --test conformance_coverage` before `nv verify`. See `crates/nvs-stdlib/tests/corpus/mod.rs`. `until: gone crates/nvs-stdlib/tests/corpus/mod.rs:Attribution`
