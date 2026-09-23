- **A `Qual` classification is enforced on the `tainted` axis only, so a `secret` claim has to be
  probed first.** `nvs_types::expr::quals::admits_tainted_argument` reads a parameter's mark for a
  tainted argument; nothing reads it for `secret`, which is refused at every mark exactly as an
  unclassified parameter refuses it. Probe with a scratch `.nvs` under `.agent-tmp/` run through
  `./target/debug/nvs.exe run <path>`, which prints every diagnostic for every line at once.
  [until: reviewed 2026-09-06]
