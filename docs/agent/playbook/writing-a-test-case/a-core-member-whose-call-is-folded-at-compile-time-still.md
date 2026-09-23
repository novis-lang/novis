- **A `Core` member whose call is folded at compile time still owes a measured figure.**
  `nvs_stdlib::attributes`' module doc says both retrievals are answered by `nvs check` and that
  their symbols abort if reached, which reads as a `[skip] perf` entry — but
  `benches/members/lang/attributes/reading-attributes-back-core-attributes-get-and-all.nvs` has
  always measured those reads, because materializing the folded constant costs a running program
  something. Before writing `[skip] perf`, list `benches/members/lang/<topic>/` for a bench over the
  same surface. [until: reviewed 2026-09-20]
