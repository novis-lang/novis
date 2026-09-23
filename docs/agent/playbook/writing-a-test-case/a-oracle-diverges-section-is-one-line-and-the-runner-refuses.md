- **A `--ORACLE-DIVERGES--` section is one line, and the runner refuses the case at the parse
  otherwise** (`line N: `--ORACLE-DIVERGES--` is one line`). The standing cases read as paragraphs
  because they are one very long line that a viewer wraps, so a divergence written as prose with
  blank lines looks exactly like them on disk and fails before anything runs. Write it as one line
  from the start. [until: gone crates/nvs-test/src/case.rs:`--ORACLE-DIVERGES--` is one line]
