- **`python tools/dossier.py --bless` runs `target/release/nvs.exe` and rebuilds it whenever the
  tree has moved, so a bless that follows a Rust edit pays a whole release build.** Its staleness
  check is the tree rather than the crate the example touches, so blessing one member's examples
  after splicing that member's Rust test in costs the build again for the next member. Write every
  example, attack and bench of a group first and bless them in one call, then splice the Rust tests
  in afterwards. [until: reviewed 2026-09-20]
