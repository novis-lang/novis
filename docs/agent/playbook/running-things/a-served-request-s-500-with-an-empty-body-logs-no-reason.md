- **A served request's `500` with an empty body logs no reason, and a memory stop looks like any other.**
  A request's ceiling is what remains of its keep-alive connection's, so a connection whose reading
  drifts stops every later request on it, with no reload and no leak. Rerun with
  `bun nv serve-mem --memory 4G`, and print the reading at `Ctx::memory_fault` in
  `crates/nvs-runtime/src/ctx/limits.rs`. `until: gone crates/nvs-runtime/src/ctx/limits.rs:fn memory_fault`
