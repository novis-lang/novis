# Handoff

## State

Goal types:enum (2/2) — 17 enums, each owing a description, three examples and one test carrying a
`// covers:` marker. An enum owes no bench and no attack; `python tools/dossier.py --id '<feature>'`
is what says so, and it is also the check, because the goal's own item list still names features that
are already finished.

Fourteen are complete: `Core\Env\Mode`, `Core\Http\Method`, `Core\IO\FileMode`, `Core\Log\Level`,
`Core\Mime\Type`, `Core\NormalForm`, `Core\Order`, `Core\SetOn`, `Core\Response\SameSite`,
`Core\Response\Redirect` and `Core\RoundMode` landed before this session, and `Core\Queue\State`,
`Core\Reflect\TypeKind` and `Core\Script\ExitReason` landed in it. Three are left: `Core\Unit`,
`Core\Weekday`, `Core\Xml\NodeKind`.

None of the three needed a new test: a marker on an assertion already on disk was the whole edit, and
a Rust `#[test]` carries one as well as a `.nvst` case does — `Core\Queue\State`'s only test is the
Rust `queue_statements_agree_with_the_state_enum`.

## Next group

**Stage 2: the last three enums, each one file set — `docs/examples/types/<Feature>/` plus the enum's
declaration and the test that pins it.** Two of the three declare in the same file. Each slice is
`rule:testing/four-proofs` as this goal's policy narrows it for an enum (`tools/data/dossier-policy.toml:20`):
`about.md`, three examples blessed with `python tools/dossier.py --bless`, and a `// covers: <feature>`
line on a test. **An example about an enum reasons over the enum and drives no backend** —
`docs/examples/types/Queue-State/03-the-check-somebody-does-every-morning.nvs` is the shape.

- [ ] **`Core\Unit`** — page, three examples, marker. It is the unit `startOf`, `endOf` and the
      `DateTime` arithmetic take, so an example is a real date question and needs no clock.
      `crates/nvs-stdlib/src/time.rs:1389`
- [ ] **`Core\Weekday`** — page, three examples, marker. Monday first, as ISO 8601 orders it.
      `crates/nvs-stdlib/src/time.rs:1470`
- [ ] **`Core\Xml\NodeKind`** — page, three examples, marker. Five kinds, both parsers produce them,
      so a walk over a parsed document is the example. `crates/nvs-stdlib/src/xml.rs:455`

## Backlog

- The goal's check re-runs eleven features that are already complete; that is the sweep doing its job
  and not a worklist — `docs/agent/loop-goal.toml:12165`.
- After the last three enums the goal is met; `python tools/verify.py --doc`, `owners.py --closes` and
  `playbook.py --closes` are the gates that come before `DONE` — `docs/agent/session-prompt.md`.
