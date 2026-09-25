- **A `Files`-style trait in a crate is the seam a filesystem question goes through, and its test
  fakes are where the question is actually asked.** Adding a method such as
  `nvs_config::resolve::Files::canonical` stops every fake compiling, and a fake that answers
  *lexically* silently makes every case a statement about paths no symlink was in — `Path::exists`
  `stat`s rather than `lstat`s, so a fake `exists` must follow links too. Grep `impl <Trait> for`
  before adding a method, and give the fake the resolving behaviour rather than the identity one.
  [until: gone crates/nvs-config/src/resolve.rs:exists]
