- **`python tools/peek.py --locate` and window targets in one call: only the locate prints.**
  `--locate` is a mode, not an extra question: every other argument is read as a symbol name, so
  windows are dropped without a word and a `file:re:pattern` target is echoed back verbatim with
  `NOT FOUND` after it, which reads like a real miss. Ask for anchors in one call and bodies in
  another, never both. [until: gone tools/peek.py:--locate]
