- **`alloc::Pooled` recycles a freed block, so a memory checker cannot see a use-after-free — and
  only one leg of four is affected.** A freed block goes onto the per-thread size-class cache
  instead of reaching `free`, so a dangling read lands in live memory and neither ASAN nor valgrind
  sees it; the leg that matters is `cargo test -p nvs-runtime`, where `cfg(test)` installs
  `counting_alloc` over `Pooled`, and `--features nvs-runtime/sanitizer` swaps in the platform heap.
  The other three legs see every free already; read `counting_alloc`'s module doc before changing
  any of it. See `crates/nvs-runtime/src/counting_alloc.rs`. [until: gone crates/nvs-runtime/src/counting_alloc.rs]
