- **`bun nv try` runs a case's `--FILE--` and drops its `--ARGS--`.** A `#[Command]` case
  therefore prints the usage page and exits 2 under `bun nv try` while passing under the real runner,
  which reads as the case being wrong. `target/covws/<host triple>/debug/nvs.exe test <case>.nvst`
  runs one case with every section honoured on the binary the driver's acceptance check uses, so it is the answer
  for any case with `--ARGS--`, `--ENV--`, `--INI--` or `--RUN--`.
  [until: exists tools/nv/cmd/try.ts:--ARGS--]
