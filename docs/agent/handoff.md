# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Stage 4's members are
whole**: all five unit tests its check names exist and pass, and its six `tests/conformance/core/`
cases are on disk. Stage 1 is goal 15's whole list; stage 5 is untouched.

**`jsonAs<T>()` has a `T` to be tested against now.** `crates/nvs-stdlib/src/request.rs:5207`'s
`reading_class` builds a class carrying a derived JSON codec out of the three pieces the runtime
exposes — `ClassTable::define`, `set_codec`, `set_methods` — with a native constructor of the ABI a
compiled one has, because `nvs_runtime::construct` faults on a class with no `CONSTRUCTOR` row.

**Stage 4's last artefact is a reject case whose refusal does not exist yet.**
`rule:security/derived-codec-qualifiers` puts the qualifier question at the call site that decodes,
and nothing asks it: a `Core\Request::jsonAs<Author>()` over an `Author` declaring a plain
`public string $name` compiles today and reaches run time. `Core\Json::decodeAs` needs no such pass —
its `$json` parameter is plain `string`, so a tainted argument is already `E0401` at the argument —
which is why this one is `jsonAs`'s alone and narrow.

**The driver's `c1e0e69e5` was read and kept**: it makes `nvs lsp` accept the `--stdio` a language
client appends unasked, and adds the two `editors/vscode` protocol tests over it. `verify.py` runs
that suite, so this session's run is the verification it never had.

## Next group

**Stage 4: the call-site qualifier refusal, and the reject case that pins it** — one file set:
`crates/nvs-types/src/derive.rs`, `crates/nvs-types/src/check.rs`,
`crates/nvs-diagnostics/src/lib.rs`.

- [ ] **A diagnostic for a `T` whose text fields are unqualified**, declared after
      `crates/nvs-diagnostics/src/lib.rs:3225`'s `E_CALLABLE_CALL_ARITY` — the `E08xx` band, next
      free `E0810`. It names the property, not the call, and its help is "write `tainted` on it".
      `rule:security/derived-codec-qualifiers`.
- [ ] **The pass**, beside `crates/nvs-types/src/derive.rs:608`'s `check_row_sites` and run from
      `crates/nvs-types/src/check.rs:213` the same way: every `CodecTy::Str` field of the class a
      `Core\Request::jsonAs<T>` wrote must declare `tainted`, since the body is the peer's bytes.
      The site list is `crates/nvs-types/src/lib.rs:458`'s shape and the class comes from
      `crates/nvs-types/src/expr/args.rs:1456`'s `written_class_of`; the module-doc gap 3 at
      `crates/nvs-types/src/derive.rs:82` is the paragraph this widens, so rewrite it.
      `rule:security/derived-codec-qualifiers`.
- [ ] **`tests/conformance/reject/a-json-body-hydrated-into-an-unqualified-type-is-refused.nvst`**,
      the last case `docs/agent/loop-goal.toml:5497`'s stage-4 suite names.
      `tests/conformance/core/a-request-hydrates-its-body-into-a-declared-type.nvst` is the accepted
      twin — the same class with `tainted string $name` — so the reject case is that file with the
      qualifier dropped and an `--EXPECTF-ERROR--` section.

## Backlog

- `examples/json-body.nvs` — stage 5's runnable fixture, six `want` lines at
  `docs/agent/loop-goal.toml:5517`, run under `nvs run --request`.
- `python tools/reference.py --check` — stage 5's one-file reference, regenerated from
  `nvs meta --json` after two new `Core\Request` rows (`docs/agent/loop-goal.toml:5534`).
- Goal 16's spec § 15 roster and exclusivity sentence, per the goal's standing decision on which
  documents this goal may amend.
