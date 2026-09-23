- **A test you are about to write may already have its name fixed by `loop-goal.toml`, and the
  handoff item will not carry it.** A gate written to the item alone lands under a fourth name and
  leaves the stage red with everything implemented; the fixed names are also a design, since three
  names say what is asserted separately. Before writing a new test for a goal item, `grep -n -A8
  'stage = "<the stage>"' docs/agent/loop-goal.toml` and take the names from the `tests = [` block.
  [until: reviewed 2026-09-06]
