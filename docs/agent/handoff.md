# Handoff

## State

**M8 goal 5. The stage 0 catch-up item is closed and its acceptance check passes.** `nvs_array_new`
hands every caller on a thread the same header, so `[]` after a thread's first is a refcount bump;
`crates/nvs-runtime/src/array.rs`'s module doc § *an empty array is a per-thread singleton* owns the
decision and what it spends. `NvsArray::new` still allocates — only the extern primitive moves, and
its Rust-side callers (`make_unique` first) take a handle they mean to write through.
`docs/perf/userland-gap.md` § K is priced by count rather than by clock, which is why the "delete
this row if the first number is noise" escape it carried never applied. `nvs_runtime::
prime_empty_array` is how a `live_bytes` balance outside this crate takes the singleton before its
window opens; the playbook bullet is the general shape.

**The `resolve()` passes were audited, and the finding is one rule rather than one mechanism.**
`Snapshot::retype`'s doc § *The seam every `resolve()` pass is measured against* is its home:
`db::canonicalize` writes the table too, `secret::materialize` carries its value beside the table and
re-applies it below, and `app::canonicalize` writes only the tree because `Snapshot::build` reads
`resolved.config.app` directly and then drops `app` from the table — three answers to three different
questions, since a secret may not reach the table and the roster may not survive into a per-app
snapshot. What they share is the rule a fourth pass is checked against, and `Snapshot::table`'s own
doc now says it is the authoritative half. The five read-only passes rewrite nothing and cross no
seam; `retype` is reached from exactly two places, both after `build` removed `app`.

**The standing acceptance failure is stage 9's `an_open_host_matching_no_grant_is_a_diagnostic`**,
gated on `nvs_types::intrinsics`' known gap 6's two obstacles, which that gap names. Nothing else is
open on this goal that the checks report.

## Next group

**One file set: `crates/nvs-types/src/intrinsics.rs` with `crates/nvs-types/src/lib.rs`.** The third
slice is where it widens, to the diagnostic registry and one conformance case.

- [ ] **Let an `Intrinsic` address a field inside a shape literal, not only a written argument
      position.** ADR 0135 § 3 merges `Core\Db::open`'s `host` into its own slot and ADR 0067 § 3
      makes that field the sink, so the row that classifies it has nothing to point at today.
      `crates/nvs-types/src/intrinsics.rs:132` is the struct,
      `crates/nvs-types/src/intrinsics.rs:36` the gap list it half-closes.
- [ ] **Carry the granted capability set on the checking environment.** ADR 0067 § 3's `db.open`
      roster is what a host is compared against, and `crate::Env` carries no capabilities at all —
      the second of gap 6's two obstacles. `crates/nvs-types/src/lib.rs:356`.
- [ ] **The diagnostic and the case the acceptance check names.** `E0618` is the next free `E06xx`
      and `crates/nvs-diagnostics/src/lib.rs` is the whole registry;
      `an_open_host_matching_no_grant_is_a_diagnostic` is the test, and
      `crates/nvs-types/src/intrinsics.rs:36` loses its gap-6 entry in the same commit.

## Backlog

- `nvs config dump` prints the resolved `path` and `tls_ca_file` now; no case asserts it —
  `docs/adr/0103-configuration-is-a-tree-of-files.md` § 9.
- ADR 0067 § 10's unterminated-literal disagreement between `nvs_types::intrinsics` and
  `nvs_db::sql` — `crates/nvs-types/src/intrinsics.rs` known gap 5.
- § K's clock number, if a bench ever wants one beyond the allocation count —
  `docs/perf/userland-gap.md` § K.
- `docs/perf/userland-gap.md` §§ D–J are the userland-gap rows still unlanded.
