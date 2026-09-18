# Handoff

## State

Goal `types-enum-2-2` — types:enum (2/2) — is met. All 17 enums have a description, three examples
and one test carrying a `// covers:` marker, and `python tools/dossier.py --verify --only …` over
the goal's list reports nothing owed with 51 examples green. An enum owes no bench and no attack;
`python tools/dossier.py --id '<feature>'` is what says so.

`Core\Unit`, `Core\Weekday` and `Core\Xml\NodeKind` landed this session, each attributed by a marker
added to a case already on disk rather than by a new test. The three gates a goal meets only at its
end are green: `verify.py --doc`, `owners.py --closes` and `playbook.py --closes` all report nothing
owed here.

**`python tools/verify.py` is red at its `test` leg, on two tests nothing in this goal touches**, both
of which the tool re-ran and reports as passing alone:
`crates/nvs-host/src/net.rs:2138` and `crates/nvs-server/src/serve.rs:5379`. The four `.nvst` cases
this session edited were run on their own and pass. Nothing here changed a line of Rust.

## Next group

**Goal `types-enum-2-2` is met, so the driver's goal switch installs goal `types-exception`'s own
generated handoff over this one.** Its first two slices, so the next session does not re-derive
them — an exception owes the hostile proof an enum does not, on top of the page, the three examples
and the marker:

- [ ] **`ArithmeticError`** — page, three examples, an attack and a `covers:` marker.
      `crates/nvs-hir/src/errors.rs:104`
- [ ] **`Core\Cli\NotInteractive`** — the same four, and a neighbour in the same file.
      `crates/nvs-hir/src/errors.rs:106`

## Backlog

- The two load-sensitive tests above owe a fix each, in a commit of its own; the playbook bullet
  under *Running things* carries what each one leans on.
- The rows in `time-datetime-startof-and-endof-are-one-agreement-over-every-unit.nvst` could now be
  one `array<Core\Unit>` loop; left written out so a case dropped from the enum fails the compile.
- No example anywhere reaches `Core\Xml\Reader`, the stream half of the same node family —
  `crates/nvs-stdlib/src/xml.rs:500` owns it.
