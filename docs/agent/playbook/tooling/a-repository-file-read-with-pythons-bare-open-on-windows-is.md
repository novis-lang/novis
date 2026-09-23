- **A repository file read with Python's bare `open()` on Windows is decoded as cp1252, not UTF-8, so
  one em dash in a fixture raises `UnicodeDecodeError` mid-probe.**
  `json.load(open('crates/nvs-stdlib/tests/vectors/webcrypto.json'))` fails exactly that way, and it
  reads as a corrupt fixture rather than as a default nobody in this tree chose. Pass the encoding —
  `io.open(path, encoding='utf-8')` — in any throwaway `python -c` that reads a file out of the tree,
  which is one more reason to reach for `tools/peek.py` whenever the thing you want is nameable as a
  target. [until: reviewed 2026-09-12]
