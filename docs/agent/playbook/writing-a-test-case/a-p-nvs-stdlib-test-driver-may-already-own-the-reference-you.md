- **A `-p nvs-stdlib` test driver may already own the reference you release, and the double release
  panics one crate away with `attempt to subtract with overflow`.** `parts_of` takes `files: Value`
  by value and ends with `files.release()`, so a test adding `dropped(files)` panics in
  `nvs_runtime::object::drop_one`. Read the tail of any `Value`-taking helper first; its
  `#[expect(unsafe_code, reason = …)]` says whether it releases its argument.
  [until: gone crates/nvs-stdlib/src/request.rs:fn parts_of]
