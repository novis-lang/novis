- **A table the compiler builds and a running program must read does not cross by adding a
  dependency; it crosses as a second, runtime-side table.** `nvs-runtime` is the bottom of the crate
  tree, so `grep RouteTable` over `crates/` answering "compiler only" is a decision, argued in
  `nvs_runtime::commands`' module doc § *Why the table is a runtime value at all* and copied by
  `nvs-cli`'s `runtime_commands` as strings plus one closed enum. Routes, jobs or any later table
  take that shape; check for the sibling before designing the edge. [until: gone crates/nvs-runtime/src/commands.rs:Why the table is a runtime value at all]
