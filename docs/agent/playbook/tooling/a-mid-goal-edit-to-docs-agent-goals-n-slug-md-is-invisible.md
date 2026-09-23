- **A mid-goal edit to `docs/agent/goals/<n>-<slug>.md` is invisible to the loop until the same edit
  lands in `docs/agent/loop-goal.md`.** That file is `goal-switch.py`'s verbatim copy of the goal's
  prose and the one `orient.py` pipes into a session, and nothing re-syncs the two between switches —
  so a settled standing decision can sit in the source while every pack still prints the question.
  Edit both, and `diff` them before committing. [until: gone docs/agent/loop-goal.md]
