# Handoff

## State

**Goal `plain-comments` — every landed comment reads plainly — has just started; nothing of it has landed yet.** Goal `the-description-is-owed`'s whole list is this goal's Stage 1 floor.

Settled before the first session: the rule is `docs/examples/README.md` § *How a comment is
written*, the bounds a script can judge are `comment_problems` in `tools/dossier.py`, and neither is
reopened here. This goal changes comment lines in programs already on disk and nothing else. The
gate's switch, `[all] comments = true`, is **off** and stays off until Stage 2 is green.

## Next group

**Stage 2: the sweep** — one file set per slice: a chapter's or a class's directories under
`docs/examples/`, `tests/hostile/` and `benches/members/`.

- [ ] `python tools/dossier.py --comments docs/examples tests/hostile benches/members` — the whole
      worklist, one program per block, one line per problem. Its last line is the count.
- [ ] Cut it by chapter and class, hand each to a worker under the rules in the goal's
      *Running this goal wide*, and launch them in one message.
- [ ] After every worker has stopped: the `git diff -U0` read for a changed line that is not a
      comment, `--run examples` over what changed, `bun nv verify`, the wrap.

## Backlog

- **Stage 3: the gate** — `tools/data/dossier-policy.toml` alone. Add `comments = true` under `[all]`
  once Stage 2's check is green, and `python tools/dossier.py --gate` then passes on a line naming
  plain comments. One small slice; take it in the session that finishes the sweep.
- When this goal's last check goes green the driver takes goal `ci-green`.
