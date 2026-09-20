# Handoff

## State

Goal `lang-attributes` is **met**: `python tools/dossier.py --verify --group lang:attributes` reports
nothing owed and both suites at 0 failed, and all 8 features of the group carry their feature proofs.
This session landed the last one,
`core-json-derive-and-core-json-field-a-class-with-a-json-codec`. `python tools/verify.py` is 14 of 14
green, `--doc` resolves every link, and `owners.py --closes lang-attributes` and `playbook.py --closes
lang-attributes` both own nothing. Nothing is blocked.

Two things this feature's proofs settled. **The four `json-derive` conformance cases that already
existed are what this feature owes for tests** — a language feature is attributed by a `covers:`
marker alone, so adding the marker to a case that already pins the behaviour is the whole edit, and
`rule:testing/proof-attribution` is where that lives. And **`decodeAs<T>` survives the documents an
attacker sends**: a one-million-character string field, fifty thousand undeclared keys, a hundred
thousand open brackets, a truncated document and ten thousand repeated decodes all leave the runtime
standing, at
`tests/hostile/lang/attributes/core-json-derive-and-core-json-field-a-class-with-a-json-codec/01-documents-a-client-sent-to-break-the-decoder.nvs`.

The figure recorded for this feature measures `Core\Json::encode` of a two-field class: 185 ns/op, 8
allocations, 440 bytes per round. It goes stale when `docs/reference/lang/90-attributes.md` is edited,
since that chapter is what every `lang:attributes/…` feature is implemented at, so a slice that
corrects the chapter re-measures with `--record-perf --only` before it wraps.

## Next group

**The goal is reached, so the next session opens the next goal in the chain** — one file set:
`docs/agent/loop-goal.toml`, `docs/agent/loop-goal.md`, `docs/agent/goals/`.

- [ ] **Take the next goal's first item** — every check for `lang:attributes` passes, including the
      stage-2 one at `docs/agent/loop-goal.toml:12386`, so the driver switches goals and rewrites
      this file before the next session reads it. Nothing from `lang-attributes` is left open, and
      `rule:testing/feature-proofs` is what the next generated goal runs on as well.

## Backlog

- The reference chapter's `#[Core\Json\Derive]` section says a decode of a class with an array or an
  enum field is a fatal error at run time; nothing pins that, and it is a `# Known gaps` item for
  `crates/nvs-stdlib/src/json.rs` rather than a proof this group owes.
- `python tools/dossier.py --bless <dir>` fails with a raw OS error instead of saying it wants files;
  pass the `.nvs` paths. `tools/dossier.py` owns it.
