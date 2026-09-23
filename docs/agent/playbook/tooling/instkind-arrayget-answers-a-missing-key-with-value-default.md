- **`InstKind::ArrayGet` answers a missing key with `Value::default()`, so a lowering that treats
  the result as a pointer aborts.** The symptom is *"an Novis array pointer is never null"* then
  *"panic in a function that cannot unwind"* and exit 127 inside `nvs_array_set`, which reads as an
  array-module bug; `nvs_array_get`/`nvs_array_get_index` both end in `.unwrap_or_default()`, which
  is the recognition test. A borrowing read is no building block for a write path: use a helper
  whose ownership answer is the same in both cases, as `Helper::ArrayRowForWrite` retains what it
  found or allocates what it did not.
  [until: gone crates/nvs-runtime/src/array.rs:unwrap_or_default]
