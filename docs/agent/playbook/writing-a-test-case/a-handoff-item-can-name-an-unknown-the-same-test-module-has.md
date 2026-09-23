- **A handoff item can name an unknown the same test module has already answered, and the fixture is
  usually a few lines below the anchor it gave you.** A member that landed with tests brought its
  fixtures with it — a context granting a capability through a `nvs_config::Snapshot`, a scratch
  directory under `std::env::temp_dir()` — so an item predicting a setup cost is often already paid.
  One `grep -n 'fn ' crates/<crate>/src/<file>.rs | awk -F: '$1>NNNN'` over the test module lists
  every fixture it holds, and it is the first call to make. [until: reviewed 2026-09-06]
