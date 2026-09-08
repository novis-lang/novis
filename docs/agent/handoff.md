# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Stage 5's pages are on disk.**
`Core\Request` had a hand-written page and none of the three classes it hands out did.
`docs/reference/core/Request-BodyStream.md`, `Request-Files.md` and `Request-Part.md` are written and
`docs/novis.md` is regenerated around them; `python tools/reference.py` reports 289 of 289 examples hold.
Nothing here is waiting on a decision.

**The acceptance check red since session 0002 was a name, not missing work.** The check asked for
`a_non_file_part_is_buffered_into_post`; the claim is pinned by
`post_reads_the_fields_a_files_walk_buffered` (`crates/nvs-stdlib/src/request.rs:5062`), over a body whose
last field is written *after* the upload, with the walk's own side at
`a_files_walk_yields_the_file_parts_and_drains_what_it_passes` (`crates/nvs-stdlib/src/request.rs:4140`).
Both copies of the check — `docs/agent/loop-goal.toml` and `docs/agent/goals/16-request-json.toml` — now
name the test that exists. This is the playbook's "a drafted name describes the claim, which is usually
already pinned under the name the corpus took", one file type over.

**Two `Core\Request` classes still render from their reference cards alone**, which is the state the three
pages above replaced: `Core\Request\Mount` and `Core\Request\PartContent`. That is the next group.

## Next group

**Stage 5: the two `Core\Request` classes still rendering from cards alone** — one file set:
`docs/reference/core/`, `docs/novis.md`, `crates/nvs-stdlib/src/request.rs`.

- [ ] **`docs/reference/core/Request-Mount.md`** — where the router put this request and what a mount is
      not: `rule:http-server/a-mount-carries-no-policy` and `rule:http-server/a-mount-table-expands-at-boot`
      are the two rules, the class is `crates/nvs-stdlib/src/request.rs:773` and its `captures` card is
      `crates/nvs-stdlib/src/request.rs:813` — one compiled program at many prefixes learning which tenant
      it answers for. `docs/reference/README.md:97` is the fence grammar, and a member needing a request
      goes in an `nvs skip` fence with the runnable `LogicError` refusal beside it, as the three pages
      landed this session do.
- [ ] **`docs/reference/core/Request-PartContent.md`** — the chunk walk `Core\Request\Part::content()`
      answers, which `Request-Part.md` names and no page introduces: the class is
      `crates/nvs-stdlib/src/request.rs:1260`, its second slot is the part ordinal the walk is checked
      against, and `rule:http-server/a-part-is-consumed-in-one-of-three-ways` is what makes it one of the
      three. Its shape is `Request-BodyStream.md`'s, one level down.
- [ ] **Regenerate and prove them in one call** — `python tools/reference.py` writes `docs/novis.md` and
      runs every example; what it reads a page for is `tools/reference.py:21` and the fence it runs is
      `docs/reference/README.md:97`. The pages and the regeneration land in one commit, because a page
      whose class is not yet in `docs/novis.md` fails `reference.py --check` on its own.

## Backlog

- `docs/agent/goals/34-workspace-index.*` and `35-editor-surfaces.*` landed unverified as `f4198b8d3`
  (the driver's wip commit) and `a422c7269` amends them; nothing has run `plan.py --check` over the chain
  since. Owner: `docs/agent/goals/chain.toml`.
- `[limits] upload_total` is a constant in `crates/nvs-server/src/body.rs:61`, not an `nvs_config` row;
  `Request-Files.md` says so in one clause and will need editing when the configuration slice lands.
  Owner: `rule:http-server/request-body-and-upload-total-are-two-caps`.
- No page for `Core\Router\Match`'s siblings was checked this session; `docs/reference/core/` is complete
  for `Core\Request` after the group above. Owner: `docs/reference/README.md`.
