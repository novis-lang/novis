- **A refcount cycle that is still live at exit is a `definitely lost` under valgrind and always
  will be — read the fixture before you read the runtime.** Novis refcounts and has no cycle
  collector (`crates/nvs-runtime/src/object.rs` says so), so a self-referential object's last
  reference is its own field and dropping the local frees nothing; `examples/serialize.nvs` breaks
  its rings by hand before it ends. A leak stack whose top frame is an object allocation
  (`nvs_object_new`) is the shape to suspect — grep the `.nvs` for a cycle before opening the Rust.
  [until: reviewed 2026-09-06]
