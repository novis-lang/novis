# Handoff

## State

**Goal 6, M7. ADR 0102 § 7 is closed, both halves.** The door carries the prefix and the captures
(`nvs_server::mount::carry`), and `Core\Request::mount()` answers them as `Core\Request\Mount` — a
two-slot instance class whose `prefix()` is plain and whose `captures()` is
`array<tainted string>`. Both acceptance checks the split produced are green.

**The return-type question is decided and recorded.** § 7's shape literal has no return-position
spelling — `CoreTy::Shape` is parameter-only and a `Ty::Shape` value erases to `Ty::Object`, taking
the qualifier with it — so the pair is a class, exactly as `Core\Router\Match` already is. The
reasoning lives once, in `crates/nvs-stdlib/src/request.rs`'s `MOUNT` doc; § 7's body and spec § 15
were folded to spell the class in the same commit, `spec-members-part-two-outstanding.txt` lost its
`§15 Request::mount` line, and `nvs-types`' two taint rosters gained the two new rows.

**`Core\Request` is thirteen readers now**, and the agreement case that asks all of them whether a
program answering no request is refused was widened with it.

**What ADR 0086 § 6 still owes is corpus, not code.** `ArgConv::OneOf` converts and refuses in
`crates/nvs-stdlib/src/command.rs`; no `.nvst` case asks it anything, where the enum half landed
with two.

## Next group

**§ 6's union in the corpus, over one file set:** `tests/conformance/core/command-run-*.nvst` and
`crates/nvs-stdlib/src/command.rs`.

- [ ] **A `OneOf` argument converts by its word** (ADR 0086 § 6) — the arm is
      `crates/nvs-stdlib/src/command.rs:543` and the variant it reads is
      `crates/nvs-types/src/commands.rs:184`; the shape to copy is
      `tests/conformance/core/command-run-converts-an-enum-argument-by-its-case-name.nvst`, and
      `crates/nvs-stdlib/src/command.rs:1186` is a table already declaring a two-member union.
- [ ] **A word outside the set is a usage error naming every accepted one** (ADR 0086 § 6) — the
      refusal sibling, whose shape is
      `tests/conformance/core/command-run-refuses-an-enum-argument-written-as-its-backing-value.nvst`;
      the message is written at `crates/nvs-stdlib/src/command.rs:543`'s arm, so read it there
      rather than guessing the wording.
- [ ] **A subset of an enum's cases still refuses to convert** (ADR 0086 § 6) —
      `crates/nvs-runtime/src/commands.rs:100` is where `OneOf` and the enum meet, and gap 1 in
      that module's own doc states what closing it needs. Take this only if the two above left the
      file set loaded and the budget open.

## Backlog

- § 4's CSRF refusal needs a verified token and a `[http]` key — `crates/nvs-server/src/route.rs`'s
  known gap 1.
- Nothing exports ADR 0076's `route` label — `crates/nvs-server/src/route.rs`'s known gap 2.
- Raw body access for an arbitrary content-type — ADR 0024's *Revisiting*, narrowed by
  `docs/plan/m7.md`.
- A capture's text is still percent-encoded — `nvs_runtime::routes`' own gap 4.
