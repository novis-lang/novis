# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32, 33 and 34 are closed**; 35 is the only one left.

**Item 34 closed with D10.** `nvs_types::Route` carries ADR 0085 § 2's four values — `tags`,
`security`, `errors` (a `u16` status with the resolved class) and `example` (folded to its ADR 0046
§ 5 constant while the declaring file's imports are still in reach) — because `check_api`'s four
walks each hand back what they accepted, so a value § 2 refused reaches no row. `nvs-cli`'s emitter
writes them, § 1's `200` outranking an `errors` entry that names it. **That module's gap list is
the one home for what the document still lacks**, and it is four entries now: gap 3 became
`components.securitySchemes`, which is the same missing configuration surface `check_api` records
for the half of § 2's scheme rule it cannot ask.

**Item 35's acceptance check is green** (M10's `no_registry_card_cites_an_adr` in
`crates/nvs-stdlib/src/registry.rs`); its other halves have no check and are still open: D3, D4,
D2, D6, D9, D20, D29, U13, M2, M3, M4, and the two `docs/reference/lang/` chapter fixes.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1077,
differential 206. The driver's standing acceptance failure is stage 8's
`a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst`, one of the two isolate cases
that were never written; it is an open item, not a regression, and it outranks nothing here.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`. The next free `E02xx` is `E0247` and `E07xx` is
`E0794`; this session added no diagnostic.

**`orient.py`'s `[context]` gaps.** This session needed, in `modules`,
`crates/nvs-types/src/routes.rs` and `crates/nvs-cli/src/openapi.rs` — the two files its own item
named, neither of them in the map. Still missing, each proven by an earlier session: **0072
§§ 6-7**, **0012 § 6**, **0013 §§ 2-4**, **0046 §§ 2, 4-5**, **0053 §§ 1-3**, **0007 §§ 2-3**,
**0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**, **0011**, **0086 § 6**,
**0096 §§ 1-1a** and **0117 § 1**; and in `modules`, `crates/nvs-types/src/enums.rs`,
`crates/nvs-types/src/lib.rs`, `crates/nvs-runtime/src/ctx.rs`, `host.rs`, `script.rs`,
`crates/nvs-stdlib/src/task.rs`, `crates/nvs-config/src/snapshot.rs`, `directive.rs`,
`crates/nvs-types/src/layout.rs`, `crates/nvs-types/src/expr/args.rs`, `retrieval.rs`,
`attributes.rs`, `serialize.rs`, `crates/nvs-hir/src/errors.rs`, `crates/nvs-ir/src/lower/call.rs`
and `crates/nvs-codegen/src/lib.rs`. `orient.py` still warns that `crates/nvs-host/src/budget.rs`
matches nothing, which is the forward anchor its own comment describes.

## Next group

**Item 35's three time cards, which are one file.** The file set is `crates/nvs-stdlib/src/time.rs`
alone; each is a card's wording against the rule its ADR states, and
[docs/reference/findings.md](../reference/findings.md) § *Triage* holds the verdict for each.

- [ ] **D3: `Core\Weekday`'s card says the cases are zero-based** — `crates/nvs-stdlib/src/time.rs:1459`
      is `WEEKDAY_DOC`, and `crates/nvs-stdlib/src/time.rs:2003` is the wording a sibling card
      already uses for the same fact.
- [ ] **D2: `Duration`'s card says `==` is identity and `compareTo` is the content comparison** —
      `crates/nvs-stdlib/src/time.rs:595`'s `DURATION_COMPARE_TO_DOC`, against ADR 0090 § 3.
- [ ] **D4: the time cards say a *literal* pattern or duration is refused at check time** — only a
      computed one throws (ADR 0057). `crates/nvs-stdlib/src/time.rs:483`'s `DURATION_PARSE_DOC` is
      the first; the `DateTime` parse and format cards are the rest, and `E0769` appears nowhere in
      the file today.

## Backlog

- Item 35's other halves: D6 (`crates/nvs-stdlib/src/router.rs:539`), D9's `#[Json\Derive]` card,
  D20, D29, U13, M2, M3, M4 and two `docs/reference/lang/` chapters — findings.md § *Triage*.
- The two unwritten isolate conformance cases, which are the driver's standing acceptance failure
  — `docs/agent/loop-goal.toml`'s stage 8 block.
- Stage 9's items 21–23, the ADR 0119 expression `catch`; resumes when stage 0c is green.
- `crates/nvs-cli/src/openapi.rs`'s gaps 1–4, chiefly a class response body (ADR 0071's codec does
  not reach the emitter) and `components.securitySchemes`.
- `crates/nvs-stdlib/src/json.rs`'s gap 2: a `decimal`, an `Instant` and an inline shape stay
  `Opaque` in a derived codec's roster.
