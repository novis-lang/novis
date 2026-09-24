- **A `verify.py` step whose partition names files outside the walked set is cached forever.**
  `Tree` hashes only what `input_paths` in `tools/verify_keys.py` walks, so a `t.part` over any other
  path is a key over nothing. Add the new step's files to the walk (`INPUT_DIRS`, `INPUT_FILES` or a
  reads tuple beside `NV_READS`), then list the step's `STEP_READS` part to see them in it.
  [until: gone tools/verify_keys.py:def input_paths]
