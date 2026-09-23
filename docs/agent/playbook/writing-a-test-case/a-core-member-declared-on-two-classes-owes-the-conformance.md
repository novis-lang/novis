- **A `Core` member declared on two classes owes the conformance floor twice, and the wrong receiver
  counts for neither.** The floor is per class; `corpus::Attribution` attributes `->member(` to the
  classes a case mentions, so three cases writing `$db->stream(` leave `Core\Db\Transaction::stream`
  at zero. Add a `$db->transaction(fn (Core\Db\Transaction $tx) => …)` half, and run `cargo test -p
  nvs-stdlib --test conformance_coverage` before `verify.py`. [until: reviewed 2026-09-06]
