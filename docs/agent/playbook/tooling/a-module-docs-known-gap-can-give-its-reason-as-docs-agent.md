- **A module doc's known gap can give its reason as "`docs/agent/loop-goal.md` § *Standing
  decisions* keeps it out of scope", and that reason expires at the next goal switch.**
  `tools/goal-switch.py` carries neither item lists nor the sentences that referred to them, so the
  gap reads as a standing decision while being a stale one. One `grep -n` of `loop-goal.md` for the
  name settles it before you treat a gap note as a decision: a reason naming a goal file is true for
  one goal, unlike one naming a rule. [until: reviewed 2026-09-06]
