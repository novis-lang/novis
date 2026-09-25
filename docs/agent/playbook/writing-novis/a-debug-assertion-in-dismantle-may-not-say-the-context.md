- **A debug assertion in `dismantle` may not say "the context releasing this object allocated it",
  and its panic aborts.** `nvs_host::isolate::finish` drops a child's `Thrown` while the parent is
  installed and a value can outlive its whole context, so the check is routinely false — and under
  `nvs_object_release`, which is `extern "C"`, a false positive is `thread caused non-unwinding
  panic. aborting`. Assert the structural invariant instead, linked on the list its stamp names, as
  `assert_linked_where_it_says` does. [until: gone crates/nvs-runtime/src/object.rs:assert_linked_where_it_says]
