# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
Items 31, 32 and 33 are closed. **Item 35's acceptance check is green**: M10's
`no_registry_card_cites_an_adr` lives in `crates/nvs-stdlib/src/registry.rs` and 43 card fields
across eleven modules now state the fact rather than naming the decision, because a card ships
*raw* through `nvs meta --json` and `tools/reference.py`'s stripping never reaches that reader.
Item 35's other halves have no check and are still open: D3, D4, D2, D6, D9, D20, D29, U13, M2, M3,
M4, and the two `docs/reference/lang/` chapter fixes.

**Item 34 owes D7's enum half and D10, and nothing else.** D7's `array<T>` half landed:
`CodecTy::List` carries the element's wire type on the new `CodecField::element` and the element's
class label on the existing `CodecField::class`, so `nvs-codegen`'s `link_codecs` resolves it
unchanged; `nvs_stdlib::json`'s `decode_list`/`decode_element` accumulate per position under
ADR 0071 § 5's dotted path — `tags.3`, `authors.0.name`. `array<array<T>>` and an `Opaque` element
stay `Opaque` on purpose: `CodecTy` is `Copy`, so `element` has no room for an element of its own,
and that erasure is written in `codec_ty`'s own arm. json.rs's gap 2 is the one home for what the
roster still lacks.

**`Core\Task::afterResponse` exists**, unchanged this session;
`crates/nvs-runtime/src/deferred.rs` is the one home for when it runs, why a throw or a `FATAL`
runs none of it, and why only the request's own task may register. **Stage 9 stands where it
stood** — ADR 0119 accepted, items 21–23 written with their anchors, nothing implemented — and
resumes when stage 0c is green. **Stage 8**: conformance 1076, differential 206.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`. The next free `E02xx` is `E0247` and `E07xx` is
`E0794`; this session added no diagnostic.

**`orient.py`'s `[context]` gaps.** This session needed **ADR 0117 § 1** (the card's own contract,
for item 35) in `adrs`, and in `modules` `crates/nvs-ir/src/lower/mod.rs` and
`crates/nvs-codegen/src/lib.rs` — the two joins a `CodecField` field travels through. Still
missing, each proven by an earlier session: **0072 §§ 6-7**, **0012 § 6**, **0013 §§ 2-4**,
**0046 §§ 2, 4-5**, **0053 §§ 1-3**, **0007 §§ 2-3**, **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**,
**0047 § 2**, **0011**, **0086 § 6** and **0096 §§ 1-1a**; and in `modules`,
`crates/nvs-runtime/src/ctx.rs`, `host.rs`, `script.rs`, `crates/nvs-stdlib/src/task.rs`,
`crates/nvs-config/src/snapshot.rs`, `directive.rs`, `crates/nvs-types/src/layout.rs`,
`crates/nvs-types/src/expr/args.rs`, `retrieval.rs`, `attributes.rs`, `serialize.rs`,
`crates/nvs-hir/src/errors.rs` and `crates/nvs-ir/src/lower/call.rs`. `orient.py` still warns that
`crates/nvs-host/src/budget.rs` matches nothing, which is the forward anchor its own comment
describes.

## Next group

**D7's enum half, and the case that closes D7.** The file set is exactly this session's:
`crates/nvs-stdlib/src/json.rs` with `crates/nvs-types/src/derive.rs`,
`crates/nvs-runtime/src/object.rs` and `crates/nvs-ir/src/lower/mod.rs`; ADR 0071 §§ 1 and 5 are
already in the pack.

- [ ] **D7b: an enum field decodes** — `docs/reference/findings.md:219` is the finding and
      `crates/nvs-stdlib/src/json.rs:76` its gap 2. `crates/nvs-runtime/src/object.rs:495` gains an
      `Enum` variant beside `List`; what it needs beside the field is the roster of accepted
      backing values, because `crates/nvs-runtime/src/helpers.rs:2502` says a case **is** its
      backing integer at run time, so the decode is a range check and a `Value::int`, not a
      construction. `crates/nvs-types/src/derive.rs:244`'s `codec_ty` is where `Ty::Enum` erases
      today (its `_` arm), `crates/nvs-stdlib/src/json.rs:1418`'s `scalar` and
      `crates/nvs-stdlib/src/json.rs:1254`'s `decode_field` are the two match sites the
      non-`non_exhaustive` enum will break, and `crates/nvs-ir/src/lower/mod.rs:686` is the join
      that carries the new half down to `nvs-codegen`.
- [ ] **The case that closes D7** —
      `tests/conformance/core/json-decodes-an-enum-field.nvst`, written beside
      `tests/conformance/core/json-decodes-an-array-field.nvst:1`, which is D7a's and the shape to
      copy. Pin `array<Role>` in the same case: `crates/nvs-stdlib/src/json.rs:1459`'s
      `decode_list` already routes an element through the element codec, so the enum arm reaches it
      for free and the two halves close together.

## Backlog

- **D10**: `#[Api]`'s `tags`, `security`, `errors`, `example` are absent from `nvs build --openapi`
  — item 34's last, and a different file set (`crates/nvs-cli`, ADR 0085 § 2).
- **Item 35's uncheckable halves**: D3, D4, D2, D6, D9, D20, D29, U13, M2, M3, M4 and the two
  `docs/reference/lang/` chapter fixes — `docs/agent/loop-goal.md` item 35 is the list.
- **Stage 9**: ADR 0119's expression `catch`, items 21–23, anchors already written.
- **json.rs gap 3**: a parameter default does not make a key optional (ADR 0071 § 4's two rows).
- **json.rs gap 4**: a hand-written `Core\Json\Codec` is not consulted (ADR 0071 § 7).
- **Stage 8**: two of eight differential suites unwritten; `docs/agent/loop-goal.md` § *Stage 8*.
