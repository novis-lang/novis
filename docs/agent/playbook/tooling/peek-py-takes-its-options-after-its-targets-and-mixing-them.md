- **`peek.py` takes its options *after* its targets, and mixing them is "unrecognized arguments"
  naming a target that is perfectly well formed.** The positional list is `nargs="*"` and closes at
  the first flag, so `peek.py A.rs:re:x --window 4 B.rs:re:y` fails on `B.rs:re:y` and reads as a
  malformed target; re-quoting, escaping the `::` and dropping `re:` all fail the same way. Put
  every target first and every `--window`/`--in` last. [until: gone tools/peek.py:--window]
