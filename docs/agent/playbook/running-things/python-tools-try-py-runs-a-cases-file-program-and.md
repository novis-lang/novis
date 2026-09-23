- **`python tools/try.py` runs a case's `--FILE--` program and materializes none of its other
  `--FILE <name>--` sections.** A probe written as a multi-file case — an `nvs.toml` and a child script
  beside the program — runs with those files simply absent, so what comes back is `could not read
  child.nvs` rather than the answer the probe was asking for. Write the extra files into `.agent-tmp/`
  with the Write tool and name them from the repository root inside the program (`.agent-tmp/child.nvs`),
  which is the directory `try.py` runs it from. [until: reviewed 2026-09-13]
