# Handoff

## State

**Goal `gap-owners`, stage 3 — the attribution pass — has finished `crates/nvs-stdlib`'s spec § 17
set (`compress.rs`, `zip.rs`, `mime.rs`), plus `xml.rs` and `ast.rs`**, on top of the modules the
previous sessions closed. **26 items still name nobody**, every one of them in `nvs-stdlib`;
`python tools/owners.py --check --reasons` is green over the 98 that are tagged.

**Four items left their `# Known gaps` block as decisions**, each with the evidence that settles it:
`compress.rs`'s response-path compression (`rule:http-server/two-deployments-and-nothing-a-proxy-owns`
says a proxy in front is a stated requirement and not a gap, and
`the_server_still_sets_no_content_encoding_of_its_own` pins it), `mime.rs`'s text-serialized formats
and zip containers (spec § 17's row defers *what deliberately has no case* to that module doc), and
`xml.rs`'s inter-element whitespace (`rule:errors/ambiguous-input-refused`). What stayed is a gap with
a decision attached, and the decision is the § *Unowned* bullet.

**`carried-gaps.md` § *Owned* lost two rows and § *Unowned* gained seven.** The `formats` and
`xml-tree` rows were struck because the gap each named is closed — spec § 17's three classes and
`Core\Xml`'s two shapes are registered (`crates/nvs-stdlib/src/xml.rs:157`,
`crates/nvs-stdlib/src/html.rs:172`, and no key for either in
`tests/spec-classes-part-two-outstanding.txt`) — which is the contract's *an entry leaves when the
gap closes*, not a renaming. What those modules still owe is now tagged in their own docs.

**`M8` is a live owner and `ast.rs` gap 1 takes it.** M8's cell still lists goals `signed-urls` and
`queue-purge`, so the milestone is in progress, and [m8.md](../plan/m8.md):85 states the typed AST in
scope by name.

**`python tools/verify.py`: green.**

## Next group

**Stage 3: the attribution pass, module by module** — one file set: `crates/nvs-stdlib`'s
reflection-shaped module docs, which the previous handoff already grouped. Owner kinds are the goal's
§ *Standing decisions*; `--untagged` is the worklist and `--check --untagged-is-an-error --reasons`
is the gate. Two things this session paid for and the next one should not: a milestone tag is right
while that milestone still carries a live goal, which the plan's own row at
`docs/implementation-plan.md:95` is the check for; and a `# Known gaps` item whose text argues why the
current behaviour is right is a decision only when a rule, a green check or the spec says so too —
otherwise it is an untaken decision, which is a gap.

- [ ] **Tag `crates/nvs-stdlib/src/reflect.rs:73`'s three items** — at
      `crates/nvs-stdlib/src/reflect.rs:73`, `:77` and `:81`.
      `rule:tooling/reflection-and-source-parsing-are-core-features` and
      `rule:security/reflection-enforces-visibility` are the two rules. Gap 1 is § 1's `*Info` roster,
      which `docs/plan/m8.md:85` names in scope. **Read the second rule before deciding gaps 2 and 3** — both
      items describe behaviour narrower than it states, which is a gap and not a decision.
- [ ] **Tag `crates/nvs-stdlib/src/debug.rs:54`'s three items** — at
      `crates/nvs-stdlib/src/debug.rs:54`, `:68` and `:75`. Gap 1 is a `secret` reaching a walk with
      no property to be declared on, so `rule:security/secret-crosses-no-boundary` is the rule to read
      first; the other two are roster shape.
- [ ] **Tag `crates/nvs-stdlib/src/regex.rs:65`'s three items** — at
      `crates/nvs-stdlib/src/regex.rs:65`, `:76` and `:81`.
      `rule:security/regex-pattern-is-a-sink` is the rule gap 1 names; gap 2 is a constant that no
      directive reaches, which is the § *Unowned* section's second arrival way.

## Backlog

- `cli.rs`, `command.rs`, `out.rs`, `test.rs`, `response.rs`, `storage.rs`, `csv.rs`, `path.rs`,
  `decimal.rs`, `math.rs`, `random.rs` — the remainder of the 26, `docs/agent/loop-goal.md`.
- `reflect.rs` gap 3's second half: a reflective write reaches storage through
  `nvs_runtime::write_erased_property` and runs no `set` hook, which
  `rule:security/reflection-enforces-visibility` requires — decide whether that is a gap item here or
  a `docs/agent/carried-refusals.md` entry.
- `carried-gaps.md` § *Owned* still holds ten rows whose owner goal is retired;
  `python tools/playbook.py --check` lists them, and each is the same closed-or-unowned call the two
  struck this session were.
