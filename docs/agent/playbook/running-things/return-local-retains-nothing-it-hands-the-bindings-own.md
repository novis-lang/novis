- **`return $local;` retains nothing — it hands the binding's own reference out and tells
  `release_all_locals` to skip that name.** Any binding `release_all_locals` was never going to
  release silently loses the retain: an `inout` parameter is a `Ty::Ref` cell, not refcounted, so
  returning it hands the caller a value with no owner, and `nvs run` prints the right answer and
  exits 127. A `return`/`release_all_locals` exemption keyed on a name has to be re-read whenever a
  new binding representation enters `Env`; an exit 127 is worth `git stash`-ing before you assume it
  is yours. [until: gone crates/nvs-ir/src/lower/mod.rs:release_all_locals]
