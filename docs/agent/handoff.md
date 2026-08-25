# Handoff

## State

**ADR 0007 § 2's `mixed` conversion rows are built *and* pinned**, so `mwl-ir` gap 20 is down to its two
non-scalar rows (`string` ↔ `bytes`, `array<T> as array<U>`) and Stage 0 is empty. The two new cases are
`tests/conformance/lang/a-mixed-value-converts-to-a-scalar-on-request.mwlt` and
`a-mixed-value-converts-into-an-enum-case.mwlt`; between them they pin both halves of the ordering rule the
plan's *Open now* records — a `Ty::Tagged` operand into a **literal** set is tested on its own runtime tag
first, an **enum** target converts to the backing scalar first (`lower/expr.rs:3052` owns why).

`python tools/verify.py` green, 1393 tests; `mwl test tests/conformance/` is 361 cases, all passing. The
`.agent-tmp/tagged-*.mwl` scratch programs are deleted — the two cases are those programs.

**The next group is the first Stage 3 work in a while**: `examples/collect.mwl` needs spec §§ 7, 8, 9, 11
and 12 at once, and `Core\Path` (§ 8) is the cheapest slice because every member is pure string algebra
that touches no disk and needs no new dependency.

## Next group — `Core\Path`, spec § 8

**Shared file set:** `crates/mwl-stdlib/src/path.rs` (new), `crates/mwl-stdlib/src/lib.rs`,
`crates/mwl-stdlib/src/registry.rs`, `tests/conformance/core/`. The spec table is
`docs/spec/01-core-library.md:624-649` — nine members plus the `Path::SEPARATOR` constant, and the rule
that every member accepts `/` and `\` alike on every platform and *emits* `Path::SEPARATOR`.

- [ ] **`crates/mwl-stdlib/src/path.rs` with the six separator-free members** — `basename`, `dirname`,
      `extension`, `withExtension`, `split`, `isAbsolute`. Copy the module shape from
      `crates/mwl-stdlib/src/json.rs`: a `pub const CLASS: CoreClass` at `json.rs:127` with one `Method`
      row per member naming its `symbol`, then `pub(crate) fn address(symbol: &str)` at `json.rs:193`
      returning this module's addresses and `None` otherwise. Register it in the same slice or the
      `symbols()` panic fires: `crate::path::CLASS` into `registry.rs:590`'s `CLASSES`, `pub mod path;`
      beside `lib.rs:191`'s siblings, and one `.or_else(|| path::address(method.symbol))` at `lib.rs:235`.
      `Path::SEPARATOR` is a `CoreConst` on the class roster (`registry.rs:491` says why it is not a
      `CoreTy` variant).
- [ ] **`join`, `normalize` and `relativeTo`** — the three that need real path algebra. `join`'s tail is
      `registry::CoreTy::Variadic` (one `array<string>` argument at the callee). `normalize` resolves
      `.`/`..` **lexically**, never touching the disk, and the spec says out loud it is not a launderer.
      `relativeTo` returns `?string`.
- [ ] **Conformance cases under `tests/conformance/core/`** — one per group above. **Never assert a built
      path literally**: `Core\Path` emits a platform separator, so normalize with
      `Core\Str::replace($p, Core\Path::SEPARATOR, "/")` or assert something separator-free, or the case
      passes the Windows leg and fails the WSL one (loop-goal § *Standing decisions*).

`orient.py`'s manifest printed everything this session needed. If the next session works `path.rs`, the
`[context] modules` list has no `mwl-stdlib` entry — add `mwl-stdlib/src/*` there, and add spec § 8 to
whatever field carries the spec slices, or it will pay for `docs/spec/01-core-library.md` by hand.

## Backlog

- `Encoding`/`Hash`/`Uuid`, then `ObjectSet`/`ObjectMap` (which need `new Core\X<T>()` to parse) — the
  rest of `examples/collect.mwl`, `docs/agent/loop-goal.md` § *Stage 3*.
- ADR 0009 § 3's `string` ↔ `bytes` conversion rows — `mwl-ir` gap 20's remainder.
- `array<T> as array<U>`'s O(n) element walk — ADR 0007 § 2 row 6, `mwl-ir` gap 20.
- ADR 0007 § 4's promotion table: `$n + $f` and `$n < $f` still fail in codegen — `mwl-ir` gap 19.
- The opaque `object` top has no representation arm — `mwl-ir` gap 21.
- `mwl_types` does not yet refuse ADR 0066 § 3's "cannot fail" `as ?T`, so `convert_or_null` panics on it.
