- **An acceptance `want` is a substring of a tool's own output, so rewording a summary line breaks a
  check in a stage you are not touching.** `owners.py`'s roster summary is read by a floor
  `[[check]]` wanting `, 0 owned by a retired goal`, and rewriting those counts into the `label: N`
  form the live stage asked for would have gone red on a stage that passed sessions ago. Before you
  reword any line a tool prints, grep `docs/agent/loop-goal.toml` for a fragment of it — a mode whose
  output is a verdict can take the new form while the human-facing roster keeps the frozen sentence.
  [until: reviewed 2026-09-13]
