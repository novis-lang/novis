# Handoff

## State

**Stage 0a is finished and deleted.** ADR 0107's surface — `inout` before the
type in all three binding positions and again at the call site, `&` in a
by-reference position `E0237`, the call-site marker enforced both ways
(`E0713`/`E0714`) — is landed in the tree, in the compiler's own identifiers and
prose, in the `.mwlt` corpus, and in `docs/`. Both trees are green at **676**
conformance and 174 differential; no session since has changed Rust.

- **What still spells `&$` in `docs/` is deliberate quotation.** ADR 0107's own
  17 sites, ADR 0031's 16 `use (&$y)`, the three refusals the plan names
  (`$a = &$b` `E0701`, `[&$x]` `E0483`, `use (&$y)` `E0224` — MWL has no
  reference at all, so ADR 0107 replaced no marker of theirs), and one row in
  `docs/adr/README.md` § *Where to look* that keeps `&$x` as a search key.
  `grep -rn '&\$' docs/` should stay at those and nothing else.
- **`docs/agent/loop-goal.md` § *Stage 0a* is gone; its `loop-goal.toml` checks
  are not.** The two `stage = "0a inout"` blocks stay as guards — their `stage`
  string must keep its leading `0` or `loop.py` moves them out of the catch-up
  class (playbook) — and their comments now cite ADR 0107 rather than the
  deleted items 44/45/47.
- **Stage 0 is what a session takes its group from next, and `holes.py` says it
  is nearly empty.** The 9 sites it still attributes to item 1 are `emit.rs`'s
  generic catch-alls, attributed by *file* rather than by shape; item 4 has one
  and item 16 two. The real frontier is Stage 3's item 13 and the ten Stage 5–7
  named cases `python tools/holes.py --cases` lists.
- **ADR 0108 landed and changes nothing before M10.** A review of DEVSENSE's PHP
  Tools for VS Code against 0016/0040/0099 found the plan level or ahead on
  every language-server request and short on the layer above: `references` was
  missing outright, and with it the four features that are the same index read
  four more ways. M10 gained six items, `mwl dap` gained a named capability
  list, and `mwl check --json` is now owed. **M4B's frozen request set and
  contributions are untouched**, so a session working the current goal can
  ignore all of it.

## Next group

**Item 13 — an abandoned generator runs the `finally` it is suspended inside —
and its two cases.** Settled in § *Standing decisions*: a resume-to-unwind entry
point on the state machine plus a release-path call to it, **not** a destructor
and not a re-opening of ADR 0028 § 2. The file set:
`crates/mwl-ir/src/lower/generator.rs`, `crates/mwl-runtime/src/object.rs`, then
`tests/`. Slice 1 is the hard one and may take the session alone.

- [ ] **The resume-to-unwind entry point.** ADR 0053 § 4 owns the state machine;
      `crates/mwl-ir/src/lower/generator.rs:321` (`lower_generator`) is where the
      three synthesized methods are built, `:103` (`lower_yield`) is what parks a
      frame, and the gap is `mwl-ir` gap 18. Record the mechanism in that
      module's own `//!` doc, and fold one sentence into ADR 0028 § 2 saying this
      is not a destructor.
- [ ] **The release path calls it.** `crates/mwl-runtime/src/object.rs:1314`
      (`dismantle`) and `:1450` (`mwl_object_release`) are where a generator
      frame dies today with nothing resumed. Valgrind the pair under WSL — this
      is a new refcount edge, so `docs/agent/commands.md`'s valgrind leg is owed.
- [ ] **The two named cases**, both listed by `python tools/holes.py --cases`:
      `tests/conformance/iter/an-abandoned-generator-runs-the-finally-it-is-suspended-inside.mwlt`
      and the oracle twin
      `tests/differential/iter/an-abandoned-generators-finally-matches-phps.mwlt`.
      PHP 8.5.9 is on `PATH` on both legs, so the oracle half is real.
- [ ] **Fallback, pre-authorized**: if the state machine cannot express the
      resume, keep the divergence, pin it in `tests/differential/` as deliberate,
      and say so in ADR 0028 § 2. Never leave it undocumented.

## Backlog

- Stage 0 item 4's one remaining site and item 16's two — `python tools/holes.py --item N`.
- Item 25, `object` as a declared type has a representation arm — 2 sites, `docs/agent/loop-goal.md`.
- The two unattributed sites in `crates/mwl-codegen/src/ty.rs:116` and `:121` — no item anchors that file.
- Nine Stage 5–7 named cases still unwritten — `python tools/holes.py --cases`.
- `docs/spec/02-php-migration.md`'s score — `python tools/check-migration.py`.
