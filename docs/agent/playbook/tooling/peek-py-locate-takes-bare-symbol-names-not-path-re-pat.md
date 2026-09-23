- **`peek.py --locate` takes bare symbol names, not `path:re:pat` targets, and a target handed to it
  answers `NOT FOUND` rather than saying so.** Its positional form accepts `file:re:pat`, so the same
  string looks like it should work under `--locate`, where it is looked up as a symbol whose name is
  that whole string. Pass a `re:` target as an ordinary positional — it prints the matching line alone
  — and keep `--locate` for names. [until: gone tools/peek.py:--locate]
