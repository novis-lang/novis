- **A test outside `nvs-runtime` cannot install a current context, so an object it allocates by hand
  is in no live list and the teardown sweep misses it.** `NvsObj::new` links into whatever
  `CurrentCtx` names, and `CurrentCtx` is not in `nvs_runtime`'s `pub use ctx::{…}`. Go the way a
  request goes: `nvs_runtime::call(entry, ctx, &[…])` around an `unsafe extern "C" fn`;
  `build_a_cycle` (`crates/nvs-host/src/isolate.rs`) is the shape. [until: gone crates/nvs-host/src/isolate.rs:build_a_cycle]
