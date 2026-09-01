# Handoff

## State

**ADR 0086 § 3's table is closed.** `Core\Cli::displayWidth` answers UAX #11 columns, over grapheme
clusters, measured on the string § 1's substitution will actually put on the screen. `cli.rs`'s
known gap 1 is gone and the module's row count is fifteen — the whole of § 3 and § 4.

**The rule and the reasoning have one home each.** ADR 0086 § 3's new paragraph beside the table
states what a program can rely on: control byte as its Control Picture, cluster not code point, `TAB`
to the next multiple of eight, `LF` ending a row so the answer is the *widest* row. What
`nvs_runtime::terminal::display_width`'s doc comment owns is the one thing the ADR does not fix — the
order the three compose in — and `TAB_STOP`'s own doc owns why eight.

**The table is the `unicode-width` crate**, in `nvs-runtime` beside `unicode-segmentation`, with
`default-features = false` because the `cjk` half is an ambient-locale answer and Novis has no
ambient locale. It was already in `Cargo.lock` as `wasm-encoder`'s, so the lock gains an edge and no
package; `THIRD-PARTY-LICENSES.txt` is regenerated. `cargo deny` is not installed here — the
playbook's new *Tooling* bullet is what that costs and what stands in for it.

**`clamp` changed under the region as well**, and that is the reason the count lives in the runtime
rather than in `Core\Cli`: it now walks grapheme clusters and counts columns through the same
`advance`, so a row of wide glyphs stops one column short of the edge instead of one past it. A row
cut against a different measure than the one the program was handed is the wrapped frame ADR 0086
§ 8's clamp exists to prevent.

**The acceptance check still names `every_part_two_spec_member_is_registered`** — stage 10's gate
over a *complete* Part II, which needs §§ 15-19. Those are `Core\Request` and its neighbours and are
goal 6's, so this check cannot pass inside this goal and is not a regression.

**Two pack gaps, both still open from last session.** `[context] modules` does not select
`nvs-runtime/src/terminal.rs`, which is where this item's own anchor was. And `[context] playbook`
filters to the *item's* anchor paths, so a group that also writes `.nvst` cases never sees the
case-authoring bullets.

## Next group

**`Cli\Text + Cli\Text` — ADR 0086 § 2's last owed piece, and `Markup + Markup` is the worked
precedent for every step of it — over `crates/nvs-types/src/expr/operators.rs`,
`crates/nvs-ir/src/lower/operator.rs` and `tests/conformance/core/`.**

- [ ] **The operator row.** `Text + Text` is `Text` and every other pairing stays refused, beside
      ADR 0024 § 5's `Markup` row that already says exactly this for the other carrier —
      `crates/nvs-types/src/expr/operators.rs:1045`, with the refusal's message at
      `crates/nvs-types/src/expr/operators.rs:949`.
- [ ] **The lowering.** Two carriers composed into one, against
      `crates/nvs-ir/src/lower/operator.rs:564`'s `Markup` arm and the object-`+` gate at
      `crates/nvs-ir/src/lower/operator.rs:647`. The carrier name is
      `crates/nvs-runtime/src/ctx.rs:199`.
- [ ] **Two `.nvst` cases** under `tests/conformance/core/`: that the sum is still a carrier `echo`
      writes raw, and that a `string` on either side is refused rather than lifted — the second is
      the one that keeps `+` from becoming the hole § 1 closed. `crates/nvs-stdlib/src/cli.rs:78`
      is the module-doc sentence to delete when it lands.

## Backlog

- Stage 7: `Core\Log` reading `[log] target` — docs/implementation-plan.md, *Open now*.
- Stage 9: TLS, `AUTH` and a `list` for `Core\Mail`/`Core\Storage` — same field.
- Stage 6's Redis legs need a reachable Docker daemon — docs/implementation-plan.md, *Blocking*.
- `[context] modules` owes `nvs-runtime/src/terminal.rs` — docs/agent/loop-goal.toml.
- `[context] playbook` owes the case-authoring bullets for a group writing `.nvst` — same file.
