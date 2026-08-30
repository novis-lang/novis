# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
Items 31, 32 and 33 are closed.

**Item 34 owes D10 and nothing else.** D7 is closed: `CodecTy::Enum` and `CodecField::cases` carry
an enum field's declared backing values from `nvs_types::derive` down to `nvs_stdlib::json`, whose
`scalar` decodes one as a binary search over that roster and an integer in the enum's own backing
type — a case *is* that integer (ADR 0010 § 6), so there is nothing to construct and no name to
resolve, and a case name on the wire is refused exactly as an undeclared number is. An
`array<Enum>` element rides the same field's roster, as a class element rides its label. json.rs's
gap 2 is the one home for what the roster still lacks: a `decimal`, an `Instant` and an inline
shape stay `Opaque`.

**Item 35's acceptance check is green** (M10's `no_registry_card_cites_an_adr` in
`crates/nvs-stdlib/src/registry.rs`); its other halves have no check and are still open: D3, D4,
D2, D6, D9, D20, D29, U13, M2, M3, M4, and the two `docs/reference/lang/` chapter fixes.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1077,
differential 206. The driver's standing acceptance failure is stage 8's
`a-child-inherits-a-narrowed-capability-and-cannot-widen-it.nvst`, one of the two isolate cases
that were never written; it is an open item, not a regression.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`. The next free `E02xx` is `E0247` and `E07xx` is
`E0794`; this session added no diagnostic.

**`orient.py`'s `[context]` gaps.** This session needed, in `modules`,
`crates/nvs-types/src/enums.rs` (the `EnumTable` a roster is read off) and
`crates/nvs-types/src/lib.rs` (the `Env` every checker pass threads). Still missing, each proven by
an earlier session: **0072 §§ 6-7**, **0012 § 6**, **0013 §§ 2-4**, **0046 §§ 2, 4-5**,
**0053 §§ 1-3**, **0007 §§ 2-3**, **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**,
**0011**, **0086 § 6**, **0096 §§ 1-1a** and **0117 § 1**; and in `modules`,
`crates/nvs-runtime/src/ctx.rs`, `host.rs`, `script.rs`, `crates/nvs-stdlib/src/task.rs`,
`crates/nvs-config/src/snapshot.rs`, `directive.rs`, `crates/nvs-types/src/layout.rs`,
`crates/nvs-types/src/expr/args.rs`, `retrieval.rs`, `attributes.rs`, `serialize.rs`,
`crates/nvs-hir/src/errors.rs`, `crates/nvs-ir/src/lower/call.rs` and
`crates/nvs-codegen/src/lib.rs`. `orient.py` still warns that `crates/nvs-host/src/budget.rs`
matches nothing, which is the forward anchor its own comment describes.

## Next group

**D10 — `#[Api]`'s four values reach the OpenAPI document, which closes item 34.** The file set is
`crates/nvs-types/src/routes.rs` with `crates/nvs-cli/src/openapi.rs`; ADR 0085 § 2 is the rule and
`crates/nvs-cli/src/openapi.rs:42` is the gap that names all four.

- [ ] **D10a: the route row carries them** — `crates/nvs-types/src/routes.rs:323`'s `Route` gains
      `tags`, `security`, `errors` and `example` beside the `summary` at
      `crates/nvs-types/src/routes.rs:375`, and `crates/nvs-types/src/routes.rs:807`'s `check_api`
      is where they are read off the attribute — it already holds it to ADR 0085 § 2's *may add and
      may not contradict*, so what is missing is only the recording half.
- [ ] **D10b: the emitter writes them** — `crates/nvs-cli/src/openapi.rs:152`'s `operation` writes
      `summary` and `description` at `crates/nvs-cli/src/openapi.rs:158` and gains the four beside
      them; `crates/nvs-cli/src/openapi.rs:72`'s `document` is the caller, and gap 3 at
      `crates/nvs-cli/src/openapi.rs:42` comes off the list.
- [ ] **D10c: a case that closes D10** — a `#[Api]`-carrying route whose emitted document shows all
      four, beside one that declares none, so the absent-key shape is asserted too;
      `crates/nvs-cli/src/openapi.rs:121` is the operation-id roster the case reads back.

## Backlog

- Item 35's remaining halves: D3, D4 and D2 share `crates/nvs-stdlib/src/time.rs` —
  `docs/reference/findings.md` § *Triage*.
- Stage 8's two unwritten isolate cases, one of which is the driver's standing acceptance failure —
  `docs/agent/loop-goal.toml` stage 8.
- Stage 9: ADR 0119's expression `catch`, items 21–23 — `docs/agent/loop-goal.md`.
- `decodeAs<T>` still refuses a `decimal`, an `Instant` and an inline shape —
  `crates/nvs-stdlib/src/json.rs`'s gap 2.
- `crates/nvs-host/src/budget.rs` names no module — `docs/agent/loop-goal.toml`'s `[context]`.
