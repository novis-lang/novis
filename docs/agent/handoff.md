# Handoff

## State

Goal types:enum (2/2) — 17 enums, each owing a description, three examples and one conformance case
carrying a `// covers:` marker. An enum owes no bench and no attack; `python tools/dossier.py --id
'<feature>'` is what says so, and it is also the check, because the goal's own item list still names
features that are already finished.

Eleven are complete: `Core\Env\Mode`, `Core\Http\Method`, `Core\IO\FileMode`, `Core\Log\Level`,
`Core\Mime\Type`, `Core\NormalForm`, `Core\Order`, `Core\SetOn` landed before this session, and
`Core\Response\SameSite`, `Core\Response\Redirect`, `Core\RoundMode` landed in it. Six are left:
`Core\Queue\State`, `Core\Reflect\TypeKind`, `Core\Script\ExitReason`, `Core\Unit`, `Core\Weekday`,
`Core\Xml\NodeKind`.

Each of the three took an existing case's marker and needed no new test. `Core\Response::addCookie`
and `::redirect` both run under a plain `nvs run`, declaring a status and a header nothing in a CLI
program reads back, so an example of either prints its own commentary and the refusal message.

## Next group

**Stage 2: three enums, each one file set — `docs/examples/types/<Feature>/` plus the enum's
declaration and the case that pins it.** Each slice is `rule:testing/four-proofs` as this goal's
policy narrows it for an enum: `about.md`, three examples blessed with `python tools/dossier.py
--bless`, and a `// covers: <feature>` line in a case. **An example about an enum reasons over the
enum and drives no backend** — `docs/examples/types/Db-Driver/03-the-same-program-on-a-laptop-and-in-production.nvs`
is the shape, and it is why a queue example needs no database.

- [ ] **`Core\Queue\State`** — page, three examples, marker. No `.nvst` names it today; try the
      marker on the Rust `queue_statements_agree_with_the_state_enum` first, since the enum policy
      wants one test and not one case (`tools/data/dossier-policy.toml:20`), and write a case only if
      the sweep still reports it owed. `crates/nvs-stdlib/src/queue.rs:2615`
- [ ] **`Core\Reflect\TypeKind`** — page, three examples, marker.
      `crates/nvs-stdlib/src/reflect.rs:434`
- [ ] **`Core\Script\ExitReason`** — page, three examples, marker.
      `crates/nvs-stdlib/src/script.rs:225`

## Backlog

- `Core\Unit`, `Core\Weekday` and `Core\Xml\NodeKind` are the goal's last three enums.
- `benches/members/types/` and `tests/hostile/types/` stay empty for an enum by policy, not by
  omission — `tools/data/dossier-policy.toml:20`.
