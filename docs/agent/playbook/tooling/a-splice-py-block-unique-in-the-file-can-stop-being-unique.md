- **A `splice.py` block unique in the file can stop being unique inside the same patch.** Blocks
  apply in order against one buffer, so an earlier block's NEW text is matched too — a one-line
  accessor copied into a second impl made `&self.columns` appear twice — and the line numbers in the
  refusal are post-edit, matching neither the file nor a `grep -n`. Widen the anchor to take the
  signature above it, rather than re-grepping a file that does not hold the duplicate.
  [until: reviewed 2026-09-15]
