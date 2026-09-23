- **A plan status field is *one* logical line, and three renderings of it disagree.** `plan.py
  --get` hands back the unwrapped line, `orient.py` re-wraps it, and on disk it is a `> `-prefixed
  blockquote wrapped at ~100 columns, so an anchor copied out of either rendering never matches the
  file. To change one sentence with the Edit tool, `grep -o` the phrase in
  `docs/implementation-plan.md` and copy the `> `-prefixed lines around it; `plan.py --set FIELD
  --from <file>` is the only way to make a field-wide change. [until: reviewed 2026-09-06]
