# Handoff

## State

**Stage 0c — the reference findings — is the open stage and runs ahead of everything else in this
goal, stage 9 included.** `docs/agent/loop-goal.md` § *Stage 0c* is items 31–35;
[docs/reference/findings.md](../reference/findings.md) § *Triage* is each item's verdict and owner.
**Items 31, 32 and 33 are closed, and item 34 is open**: thirteen of its seventeen findings are
ticked, D8 closed this session and D7 half-closed. The item-34 `[[check]]`'s twelfth case now
exists and passes; its first unwritten case is the **thirteenth**,
`tests/conformance/core/script-args-are-read.nvst` (finding D12), with the fourteenth
(`task-after-response-runs.nvst`) behind it — each is one acceptance failure in the check's order.

**A `#[Json\Derive]` field may now be another deriving class, and the class identity is the
compiler's answer rather than the document's.** `nvs_types::derive::codec_ty` erases a `Ty::Class`
to `CodecTy::Class` plus the label; `nvs-codegen` resolves that label to a descriptor in
`Classes::link_codecs`, a **second pass after every class of the unit is defined**, because ADR
0071 § 2 admits a class holding a field of its own type. `ClassTable::set_codec` takes the resolved
pointers beside the field list and `ClassDesc::codec_class(i)` reads one back;
`nvs_stdlib::json::decode_nested` owns the decode. A decoder reading a class name off the JSON was
the alternative and is unsound — untrusted input would choose which constructor runs.

**ADR 0071 § 5's issue paths are a string prefix now, not a list position.** `decode_fields` takes
`""`, `2.` or `2.address.`, a nested object's issues carry their own rooted paths back up and
*join* the enclosing object's list, and `1.address.zip` is what a list of nested classes reports.
`json.rs`'s gap 2 owns what is left: an array, an enum, a `decimal`, an `Instant` and an inline
shape are still `CodecTy::Opaque`, which is a `FATAL` — at `decode_as` for the class handed in, and
at `decode_field` on reaching one inside a nested class.

**A promoted constructor parameter is a field like any other.** `nvs_types::derive::FieldDecl` is
the one view both spellings of a declaration are read through, walked in the members' own order so
the constructor's promoted parameters take their place in the encode order. `layout` and
`signatures` needed nothing — see the playbook bullet.

**`E0126` is spoken for and must not be handed out.** ADR 0119 § 3 names it in prose for the
expression-`catch` arm that refuses `return`, which stage 9 has not written yet. The next free
`E02xx` is `E0247` and `E07xx` is `E0794`; this session added no diagnostic.

**Stage 9 stands where it stood** — ADR 0119 accepted, items 21–23 written with their anchors,
nothing implemented — and resumes when stage 0c is green. **Stage 8**: conformance 1069,
differential 206.

**`orient.py`'s `[context]` gaps, still costing time.** `[context] adrs` names none of stage 0c's own
ADRs; **0071 §§ 1-2, 5, 7** is this session's proven one, and **0013 §§ 2-4**, **0046 §§ 2, 4-5**,
**0053 §§ 1-3**, **0007 §§ 2-3**, **0027 § 1**, **0031 § 3**, **0033 §§ 3-4**, **0047 § 2**,
**0011**, **0086 § 6** and **0096 §§ 1-1a** are the earlier ones. `[context] modules` misses every
file this session edited — `crates/nvs-types/src/layout.rs`, `crates/nvs-codegen/src/lib.rs`,
`crates/nvs-ir/src/lower/mod.rs` and `crates/nvs-syntax/src/ast.rs` on top of
`crates/nvs-stdlib/src/json.rs`, `crates/nvs-stdlib/src/registry.rs`,
`crates/nvs-types/src/expr/args.rs` and `crates/nvs-types/src/expr_table.rs` — and still misses
`crates/nvs-types/src/retrieval.rs`, `defaults.rs`, `consts.rs`, `expr/members.rs`, `generics.rs`,
`expr/assign.rs`, `expr/iteration.rs`, `check.rs`, `returns.rs`, `attributes.rs`, `derive.rs`,
`routes.rs`, `commands.rs`, `testing.rs`, `crates/nvs-stdlib/src/arr.rs`, `serialize.rs`,
`secret.rs`, `crates/nvs-hir/src/errors.rs`, `members.rs`, `crates/nvs-ir/src/lower/generator.rs`
and `call.rs`. `orient.py` itself still warns that `crates/nvs-host/src/budget.rs` matches nothing.

## Next group

**The item-34 `[[check]]`'s thirteenth case is D12 — `spawn script … with(args: …)` is accepted and
nothing reads it — and its file set is `crates/nvs-stdlib/src/script.rs` with the value's carrier
in `crates/nvs-host/src/isolate.rs` and the option's lowering in `crates/nvs-ir/src/lower/expr.rs`.**

- [ ] **D12 `Core\Script::args()` is `E0405`** — `docs/reference/findings.md:229`. The value is
      already carried: `spawn script`'s `args:` option lowers at
      `crates/nvs-ir/src/lower/expr.rs:2686`, `Isolate::new` takes it at
      `crates/nvs-host/src/isolate.rs:111` and holds it at
      `crates/nvs-host/src/isolate.rs:92`. What is missing is the reader — the five edits of a
      `Core` member on `Core\Script`, whose class and helpers are
      `crates/nvs-stdlib/src/script.rs:140`, and whose module doc already names this as item 22
      (`crates/nvs-stdlib/src/script.rs:52`). The design call: what a child with no `args:` reads
      back, and whether the value crosses the heap boundary the way `spawn`'s does
      (`crates/nvs-stdlib/src/script.rs:187`'s `crossing`).
- [ ] **The case D12 is closed by** — `tests/conformance/core/script-args-are-read.nvst`, which the
      item-34 `[[check]]` names at `docs/agent/loop-goal.toml:340`, over the member landed at
      `crates/nvs-stdlib/src/script.rs:140`. A multi-file `.nvst` case: the parent spawns, the child
      reads its own `args()` back and echoes them. The playbook's *Writing a test case* section owns
      what a multi-file case does and does not share.
- [ ] **D7's remaining halves — an array or enum field is still a `FATAL`** —
      `docs/reference/findings.md:219`. Same file set as the nested-class work:
      `crates/nvs-runtime/src/object.rs:484`'s `CodecTy`, `crates/nvs-types/src/derive.rs:236`'s
      `codec_ty` and `crates/nvs-stdlib/src/json.rs:1196`'s `decode_field`. An `array<T>` needs one
      level of element type beside the field (§ 2 admits no array of arrays); an enum needs its
      backing values reachable at run time, which no descriptor carries today.

## Backlog

- The fourteenth item-34 case, `tests/conformance/core/task-after-response-runs.nvst` — `docs/agent/loop-goal.toml:341`.
- A JSON *array* reaching a scalar `decodeAs<C>` still reports every field missing rather than naming the shape — `crates/nvs-stdlib/src/json.rs`'s module doc.
- ADR 0071 § 7's hand-written `Core\Json\Codec` is not consulted, nested or not — `json.rs` gap 4.
- A `#[Json\Field(skip: true)]` field with a constructor default is still an engine fault — `json.rs` gap 3.
- Stage 9's items 21–23 (ADR 0119's expression `catch`) — `docs/agent/loop-goal.md`.
- `orient.py`'s `[context]` manifest gaps, listed in `## State`.
