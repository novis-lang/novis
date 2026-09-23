- **A bare `>` line inside a plan field is the field separator, so a blank line added for
  readability splits the field in two.** `plan.py --check` then reports six fields where there were
  seven, and AGENTS.md's fixed field set forbids both the split and the eighth field. Keep the
  paragraph continuous when hand-editing `docs/implementation-plan.md`, and run `python
  tools/plan.py --check` after — it prints the field count and each field's size.
  [until: reviewed 2026-09-06]
