# Handoff

## State

**Goal `decided-closures`, stage 4 — the library.** `crates/nvs-stdlib/src/reflect.rs`'s gap 1 is
**closed and its item deleted**: `Core\Reflect\AttributeInfo` is registered, `NOT_YET_BUILT` is
empty, and every `*Info` class ADR 0019 § 1 names is on the roster. `python tools/owners.py --closes
decided-closures` names 15, down from 16; `crates/nvs-stdlib/src/lib.rs:116` is the next one in this
file set.

**An attach site travels as a folded payload, on the constants channel one station out.**
`nvs_types::layout::ClassAttribute` (member, parameter, name, `Vec<(String, ConstValue)>`) rides
`nvs_ir::ir::Class::attributes` and `nvs-codegen`'s `attribute_desc` to `nvs_runtime::AttributeDesc`
on the descriptor. The fold is `nvs_types::consts::fold_expr`'s — `fold_const` is now a one-line
caller of it — so a payload field and a class constant give one answer to `-1`, and `nvs-codegen`'s
`constant_value` is shared by both converters for the same reason.

**The roster is own-only where the constants beside it are flattened**, and that is the one thing a
caller has to know: a constant is a name a program may write on this class, an attribute is a fact
about the declaration it was written on. `rule:attributes/attach-sites-and-forms`' four sites are all
carried, and the `target`/`parameter` pair is what tells `#[X] function f()` from `f(#[X] int $n)`
without inventing a compound spelling.

**`Core\Reflect` is not `Core\Attributes`, and the bound is where they part.** A payload value that
is a class constant, an enum case or `Foo::class` resolves through the namespace the attribute was
*written* in, which no descriptor carries — so it reads as no folded value and
`AttributeInfo::field` throws the bound rather than guessing. `Core\Attributes::get` reads exactly
those, by shape and at compile time, and the second `.nvst` case asserts both sides in one file.

## Next group

**Stage 4: `crates/nvs-stdlib/src/lib.rs`'s registry gap** — one file set, the gates and the
ratchet file rather than the reflect channel: `crates/nvs-stdlib/src/lib.rs`,
`crates/nvs-stdlib/tests/spec_registry_coverage.rs`,
`crates/nvs-stdlib/tests/spec-members-outstanding.txt` and
`crates/nvs-stdlib/tests/conformance_coverage.rs`.

- [ ] **`crates/nvs-stdlib/src/lib.rs:116` — gap 1: §§ 13–20 of the registry.** The
      `Decided:` sentence at `crates/nvs-stdlib/src/lib.rs:140` is the whole instruction and is not
      re-opened: widen both gates past § 12 and seed an outstanding-members ratchet file with every
      unwritten row. `crates/nvs-stdlib/tests/spec_registry_coverage.rs` is the gate that stops at
      § 12 and `crates/nvs-stdlib/tests/spec-members-outstanding.txt` is the file it reads, which
      only ever shrinks — the seeding is a listing job over `docs/spec/01-core-library.md` §§ 13–20,
      so **read the gate first and let it tell you the spelling of a row** rather than deriving one.
      `Core\Db`'s `stream`, `streamAs` and `Connection::close` are the three rows the gap already
      names as sitting in that position. Then delete the numbered item and its `— owner:` line.
- [ ] **`crates/nvs-stdlib/src/test.rs:94` — gap 1 and gap 2.** Takes a different file set
      (`crates/nvs-stdlib/src/test.rs:94`, `:102`), so it is a group of its own unless the one above
      leaves the session well short of the ceiling: § 4's two compile errors are runtime throws, and
      `assertThrows` matches a class by name.

## Backlog

- 15 module-doc gaps still name `decided-closures`; `python tools/owners.py --closes
  decided-closures` is the live list and the goal's own gate.
- `crates/nvs-stdlib/src/db/mod.rs:231` and `:257` are two gaps in one file — a cheap pairing when
  the `Core\Db` file set is next open.
- `docs/agent/carried-gaps.md` holds what survives a goal switch; nothing was added to it this
  session.
