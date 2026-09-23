- **A local's slot is re-pointed in four places in `nvs_ir::lower`, and a rule hooked into
  `bind_local_value` catches three.** `write_back_holder` and `write_back_array` `env.insert`
  directly, and `foreach (… as inout $v)` re-points the array binding by hand, so a nested `inout`
  foreach silently left the outer array stale. `grep -n "env.insert(" crates/nvs-ir/src/lower/` is
  the whole check for any rule phrased "whenever this name is rebound". [until: reviewed 2026-09-06]
