- **A `nvs` start-cost measurement taken from the repository root is measuring this tree's
  `nvs.toml` as much as the binary.** That file is a fixture the goals keep adding to — 207 live
  directives, 32 of them `[[app]]` blocks at about 42 µs each — so resolving it costs 1.9 ms of a
  start whose whole budget is 6, and it grew most of a millisecond while a fixed budget was being
  read against it. Name the configuration with `--config` so only the binary varies, and price what
  an implicit one costs by running `nvs config dump --config <file>` against a one-line config
  before believing a start-cost guard that went red. [until: reviewed 2026-09-19]
