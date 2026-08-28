# Handoff

## State

**M4's Stage 6 is the frontier, and every *Verification* section M4's acceptance
names now exists** — ADRs 0014, 0023, 0028, 0046 and 0069 all have one. Nothing
in ADR 0028 is re-opened.

- **ADR 0028's absences are verified where the name is written**, not where a
  hook would have fired:
  `tests/conformance/reject/every-magic-method-name-this-adr-closes-is-unspellable.nvst`
  takes all six magic names in one compile as `E0111`.
- **Two of that ADR's own claims were corrected rather than pinned** — § 4's
  "annotated with each property's declared visibility", which ADR 0092 § 3's
  plaintext rendering does not print, and the trailing headingless verification
  block, now folded into `## Verification`.
- What no case can reach is § 2's **discarded** throw out of an abandoned
  generator's `finally`: it prints nothing, exactly as a `finally` that never
  ran does. `nvs_runtime::Ctx::with_pending_set_aside` is its home.

## Next group

**The three cases `python tools/loop.py --list` still calls "not written yet",
two of which are landed under other names.** One file set:
`docs/agent/loop-goal.toml` around `:955`, plus `tests/conformance/`. It reaches
no crate, so take it as a fresh window.

- [ ] **Point the two landed names at the cases that exist** —
      `docs/agent/loop-goal.toml:955` names
      `lang/an-attribute-is-retrieved-by-its-own-type.nvst`, which landed as
      `core/an-attribute-is-retrieved-by-the-shape-it-satisfies.nvst` (ADR 0046
      §§ 4-5, structural not nominal, so the *name* is the stale half), and
      `:956` names `lang/a-dump-renders-one-record-and-redacts-a-secret.nvst`,
      which landed as `core/a-dump-renders-one-record-through-one-plaintext-view.nvst`
      plus `core/a-secret-typed-property-is-redacted-wherever-it-is-dumped.nvst`
      (ADR 0092 § 5's two halves, which a single `.nvst` cannot hold — one
      compiles and one is refused). Check each pair actually covers the named
      claim before renaming rather than after.
- [ ] **Decide what `docs/agent/loop-goal.toml:960`'s
      `a-test-attribute-builds-a-table-the-runner-reports.nvst` still owes** —
      it is Stage 7's, over the surface
      `docs/adr/0079-testing-is-a-language-feature.md:62` declares — a `#[Test]`
      method and the table built while compiling. Write it if that surface is
      there, and say so in the handoff if it is not; ADR 0079 § 2's isolates are
      M5, so the runner half may not be reachable yet.

## Backlog

- A `require` whose path is not a string literal runs nothing, silently, in both
  forms — `nvs_hir::requires`' own known gap.
- ADR 0033's container axis: an `array<T>` element and an ADR 0036 shape field
  carry no `secret` bit — `nvs_stdlib::debug`'s known gap 1.
- `Core\Reflect` is what makes ADR 0022 § 3's never-written state reachable from
  outside a `lateinit` property (M6).
- ADR 0028 §§ 2-3's converter half is M11's, per that ADR's new *Verification*
  closing paragraph.
