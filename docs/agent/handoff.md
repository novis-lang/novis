# Handoff

## State

Goal types:enum (2/2) — 17 enums, each owing a description, three examples and one conformance case
carrying a `// covers:` marker. An enum owes no bench and no attack; `python tools/dossier.py --id
'<feature>'` is what says so, and it is also the check, because the goal's own item list still names
features that are already finished.

Eight are complete: `Core\Env\Mode`, `Core\Http\Method`, `Core\IO\FileMode` landed before this
session, and `Core\Log\Level`, `Core\Mime\Type`, `Core\NormalForm`, `Core\Order`, `Core\SetOn`
landed in it. Nine are left.

No existing case needed a new test written for it — each of the five was pinned already, and the
whole edit was the marker. Two of those cases print a record's own line number, so a marker inserted
at the top of the `--FILE--` block moves every number in `--EXPECT--` by one.

## Next group

**One file set: `docs/examples/types/<Feature>/` plus the enum's declaration and one conformance
case that already pins it.** Each slice is `rule:testing/four-proofs`, as this goal's policy narrows
it for an enum: `about.md`, three examples blessed with `python tools/dossier.py --bless`, and a
`// covers: <feature>` line in the case that pins it. The first two share an implementing file.

- [ ] **`Core\Response\SameSite`** — page, three examples, marker. `crates/nvs-stdlib/src/response.rs:877`
- [ ] **`Core\Response\Redirect`** — page, three examples, marker. `crates/nvs-stdlib/src/response.rs:923`
- [ ] **`Core\RoundMode`** — page, three examples, marker. `crates/nvs-stdlib/src/math.rs:1232`

## Backlog

- `Core\Queue\State`, `Core\Reflect\TypeKind`, `Core\Script\ExitReason`, `Core\Unit`,
  `Core\Weekday`, `Core\Xml\NodeKind` are the rest of this goal — `docs/agent/loop-goal.md`.
- A run with no `nvs.toml` writes `Debug` records: the per-mode `log.level` default is not applied
  at boot, which `nvs_config::mode`'s module doc carries as its own open gap.
- `docs/examples/types/*/about.md` is written but not yet counted by any check — goal
  `the-description-is-owed` switches that on.
