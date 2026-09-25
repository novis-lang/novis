- **A hostile case whose `// hostile: timeout-ms` was raised until it passed can be hiding a
  finding, and it fails the sweep again once the machine is busy.** `Core\Html::sanitize`'s attack
  declared 60s and used 52s of it alone, because a hundred thousand nested tags parse in quadratic
  time; under the sweep's eight programs at a time it timed out. Time a new attack's steps alone
  with `target/release/nvs.exe run` before you raise its budget, and when one step is what needs the
  time, move that step into its own file marked as a known gap and leave the default 10s on it.
  [until: gone crates/nvs-stdlib/src/html.rs:Core\Html::sanitize]
