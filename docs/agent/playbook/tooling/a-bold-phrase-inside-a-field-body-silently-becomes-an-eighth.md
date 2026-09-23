- **A `**Bold phrase:**` inside a field body silently becomes an eighth status field.** `plan.py`'s
  `FIELD_RE` is `^> \*\*([^*:]+):\*\*`, and `--set` re-wraps, so a bolded lead ending in a colon
  that lands at the start of a wrapped line is parsed as a new field on the next read, which
  AGENTS.md's fixed field set forbids. Write `**Bold phrase** —` instead; the symptom is `python
  tools/plan.py` listing eight fields, and the fix is `git checkout -- docs/implementation-plan.md`
  followed by the `--set` again. [until: gone tools/plan.py:FIELD_RE]
