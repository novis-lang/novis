- **`python tools/try.py` runs a case's `--FILE--` and drops its `--ARGS--`.** A `#[Command]` case
  therefore prints the usage page and exits 2 under `try.py` while passing under the real runner,
  which reads as the case being wrong. `target/debug/nvs.exe test <case>.nvst` runs one case with
  every section honoured and is the binary the driver's acceptance check uses, so it is the answer
  for any case with `--ARGS--`, `--ENV--`, `--INI--` or `--RUN--`.
  [until: exists tools/try.py:--ARGS--]
