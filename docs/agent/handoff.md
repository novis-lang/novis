# Handoff

## State

**Goal `m7-server-surface` is met.** Stage 13 flipped the last of its rules to `shipped` in the
previous session; the only thing left red was the floor check `python tools/playbook.py --check`,
and it is green now. Two bullets carried trailers the tool could not read: one ended
`[until: \`--BODY--\` crosses as octets]`, which names no kind, and one had its `[until: gone
tools/peek.py:…]` needle wrapped onto a second line, so the needle held a newline no file content
can and the condition read as *held* — `--retire` would have deleted a bullet whose trap is still
live (`tools/peek.py:313` bounds nothing).

**Both classes are now refused where they would be written, not where they are found.**
`playbook.declaration` returns `None` for a trailer broken across a line and `playbook.wrapped` says
so in one word, so `--check` reports it as malformed instead of retiring the bullet; and
`session.py --wrap` asks each bullet of a `## playbook:` section separately, because it read only
the section's last trailer and that is how the malformed one landed in front of a good one.

`python tools/verify.py` and `--doc` are green. Nothing is blocked.

## Next group

**Nothing open in this goal.** The chain's next entry is goal `m8-db-queue`, which
`python tools/chain.py` installs once this one retires; its own file and `[context]` manifest carry
the group, and this handoff is overwritten by the goal switch.

## Backlog

- `peek.py`'s `re:` target still prints every hit in the file unbounded (`tools/peek.py:313`); the
  playbook bullet about it carries a `reviewed` date now, since nothing in the tree names the bound
  a mechanical trailer could wait on — `docs/agent/playbook.md` under *Tooling*.
