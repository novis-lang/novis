# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it — is closed.** `python
tools/loop.py --goal-only` reports `GOAL REACHED: every acceptance check passes`, 485 checks in 482s.
`python tools/verify.py` is 8 of 8 green: 3598 tests, 1603 conformance, 276 differential, 291 of 291
examples. Nothing is blocked. The driver's next step is to install the next chain entry, whose seed
handoff replaces this file.

**What was left of the goal was a check name, not code.** Stage 2's conformance check named three
`.nvst` paths the corpus never took; the two cases that pin those three claims landed at `4588e8449`
under names describing the claim rather than the section carrying it. Both copies of the check now
name them (`docs/agent/loop-goal.toml:5439`, `docs/agent/goals/16-request-json.toml:5420`) — the same
repair `807635195` made for the upload check, and the one `rule:testing/nvst-is-separate`'s corpus
habit implies.

**The carried link signal is closed too**: `python tools/check-links.py` reports every link in 3937
files resolving. `loop.py`'s `relocate_links` (`tools/loop.py:3373`) rewrites a seed handoff's
relative links when it installs one, so a seed cites from where it *sits*, never from where it lands.

**The dirty paths under `docs/agent/goals/` are the user's**, not a session's — goal 36 is being
authored in this tree while the loop runs. Stage your own paths explicitly; the new playbook bullet
under *Tooling* has the rest.

## Next group

**Nothing of goal 16 remains**, so the next group is the next chain entry's, installed by the driver
along with its own seed handoff. If you are reading this because the switch has not happened, this is
the one item worth taking — one file set: `docs/implementation-plan.md`.

- [ ] **`Open now` is 2042 B against its 2000 B ceiling**, so the next wrap that has to edit it is
      refused unless the edit replaces a sentence rather than adding one. Trim the clauses the goals
      already walked have closed, rather than growing the field.
      `docs/implementation-plan.md:46` is the field; `docs/agent/conventions.md` § *A status-block
      field* is the shape and `python tools/plan.py --check` is the gate.

## Backlog

- Goal 16's five stages are all on disk; no follow-up of its own is outstanding — `docs/agent/goals/16-request-json.md`.
- `--INI--` still parses without being honoured until `nvs.toml` is read — `crates/nvs-test/src/lib.rs:144`.
- The doc-cleanup pass is still deferred and is the user's to fire — `docs/agent/doc-cleanup.md`.
