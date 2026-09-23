- **`peek.py --locate` is a mode, not a flag you can add to a read: every `path:target` in the same
  call is silently dropped.** `--locate` takes the rest of argv as symbols, prints their anchors and
  nothing else, and a symbol it cannot find exits 1 — so the windows you batched read as files
  holding nothing. Ask for anchors in their own call, or use a `file.rs:re:fn name` target, which
  prints `file:line` and the matching line beside the other windows.
  [until: gone tools/peek.py:--locate]
