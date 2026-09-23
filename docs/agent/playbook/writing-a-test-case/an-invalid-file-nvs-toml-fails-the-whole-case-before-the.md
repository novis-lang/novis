- **An invalid `--FILE nvs.toml--` fails the whole case before the program starts: `E0601`/`E0609`
  on stderr, empty stdout.** `boot_snapshot` reads it for real, and
  `rule:config/a-duplicate-key-is-an-error-and-so-is-an-unknown-one`'s typed tree refuses an
  `[[app]]` block with no `root` or `entry`, a root-level `origin` and any unknown key. `entry =
  "nvs.toml"` is a legal key matching no program, the shape a decoy block wants.
  [until: reviewed 2026-09-06]
