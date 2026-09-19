# Handoff

## State

**Goal `lang-classes` is complete.** All 15 features carry their feature proofs, and
`python tools/dossier.py --verify --group lang:classes` reports nothing owed, 45 examples ok / 0
failed and 15 attacks ok / 0 failed. The goal's end gates are green too: `verify.py --doc` resolves
every link, and `owners.py --closes lang-classes` and `playbook.py --closes lang-classes` each name
nothing. Nothing is blocked.

The last feature, `what-a-class-cannot-declare`, is three compile errors — magic methods, anonymous
classes, nested classes — so its proofs take the shape the two `lang:expressions` refusal features
already use. The examples show the replacements (a `get` hook, `PropertyObserver`, a named class at
file scope), the attack hands the compiler all six refused shapes at once and is refused 35 times
with nothing run, and the perf proof is a recorded `[skip]` in `tools/data/dossier-policy.toml`
because there is no program to iterate. Its three tests are the cases that already pinned the
refusals, each given a `covers:` marker below its last refused line.

## Next group

**A casing refusal's suggested fix, printed twice — one file set:** `crates/nvs-syntax/src/casing.rs`
and the renderer that prints a `with_fix`. `rule:errors/diagnostic-record` owns the record these two
fields sit in.

- [ ] **A casing fix names its replacement twice in the rendered record.** `nvs run` on a class
      declaring `__get` prints `= suggestion: rename to `get`: `get``: the message built at the fix
      site already carries the name, and the renderer appends the replacement text after it. Decide
      which of the two drops it. `crates/nvs-syntax/src/casing.rs:228`
- [ ] **Two more `with_fix` calls in the same file build the same `rename to `X`` message**, so
      whichever way the first is decided lands in all three together rather than one at a time.
      `crates/nvs-syntax/src/casing.rs:270`

## Backlog

- `docs/reference/lang/50-classes.md:1251` names `__set_state` and `__debugInfo` among the refused
  identifiers without naming what replaces either; the section names replacements for the rest.
- `tests/conformance/lang/a-type-declared-inside-a-body-is-a-compile-error.nvst` now attributes to
  this feature while also pinning nested `interface` and `enum`, which other chapters own.
