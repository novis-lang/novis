- **A multi-table `delete` on MySQL is driven from a derived row, and its predicate goes on the
  join.** `delete j, d from nvs_jobs j left join nvs_dead_jobs d …` answers nothing for a job that
  has already moved tables — there is no left side to hang the other row on — and a trailing `where
  j.state <> 1` drops the driving row for a claimed job, taking that arm with it. Drive it from
  `(select ? as jid, ? as qname) r`, give each table its own `left join` carrying its own predicate,
  and prepare it against a live server before writing the doc comment.
  [until: gone crates/nvs-stdlib/src/queue.rs:delete j, d]
