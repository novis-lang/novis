- **`holes.py` keys on the refusal *phrase*, not on the macro.** A site reworded from `panic!` to
  `unreachable!` still counts as a hole while its message matches `REFUSAL` in `tools/holes.py`
  ("does not lower", "only lowers", "no lowering for"); there is no allowlist and no attribute. The
  last edit of a slice that answers a hole at the checker is to reword the assert to state what
  reached lowering and name the code that refuses it ("… reached lowering: … refuses this where it
  is written, as `E0494`"), or the worklist you quote is one higher than the tree.
  [until: gone tools/holes.py:REFUSAL]
