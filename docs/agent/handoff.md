# Handoff

## State

**Goal `unowned-closures`. The register is `unowned: 15`** (`python tools/owners.py`), `--deferrals`
green, 44 items still owned by this goal. The 15 unowned are the scheduling questions and none of
them is this goal's own gap.

**`nvs_codegen::make_executable` is the one home for publishing written bytes as code**, and the
artifact loader goes through it — `crates/nvs-codegen/src/lib.rs:@make_executable`,
`crates/nvs-cli/src/cache.rs:@Verified::relocate`. Mach-O's leading underscore is no longer a gap
either: `crates/nvs-cli/src/cache.rs:@linkage_name` strips it where a name is read.

**`aarch64` still does not load, and the reason has changed.** The `Decided:` sentence priced the
make-executable step, which is now built; what actually refuses an aarch64 payload is that this
loader speaks no instruction-field relocations — `object` reports a `CALL26` as 26 *bits* wide,
which the flat little-endian write in `Layout::apply` cannot serve. `crates/nvs-cli/src/cache.rs`'s
`# Known gaps` now says that and still names this goal as owner; the bullet is one item, not two.
Building the vocabulary is a slice nothing on this x86-64 host can execute, so it is in the backlog
with anchors rather than taken silently.

**Stage 5's remaining items are the three server-door gaps plus `nvs-runtime`'s metrics one.** The
`nvs-cli` and `nvs-db` entries in the goal prose's list are closed; the register is what is true.

## Next group

**Stage 5: the server door's three open gaps** — one file set:
`crates/nvs-server/src/{route,schedule,trace}.rs`. All three are `# Known gaps` items in modules
that run before any application code does, so the shapes and the `Ctx` wiring are shared.

- [ ] **The door refuses an unchecked cross-origin write, rather than only answering whether it
      checked** — `crates/nvs-server/src/route.rs:32` gap 1, whose rule is
      `rule:routing/matched-once-before-the-handler` for where in the door it belongs.
- [ ] **A fire's context carries the configuration its `limits` sub-cap is measured against** —
      `crates/nvs-server/src/schedule.rs:80` gap 1, under
      `rule:config/a-scheduled-run-is-a-root-isolate`, with `rule:programs/memory-priority` for what
      the ceiling is enforced for.
- [ ] **A sampled request's graph gets more than its root span** —
      `crates/nvs-server/src/trace.rs:78` gap 1, under
      `rule:observability/an-inbound-traceparent-is-continued`.

## Backlog

- The aarch64 relocation vocabulary — `crates/nvs-cli/src/cache.rs:@Layout::apply`,
  `:@landing_for`, `:@HOST_ARCH`, `:@JUMP_THROUGH_NEXT_EIGHT`; owned by this goal, unexecutable on
  an x86-64 host, so it wants its own decision before a session takes it.
- `crates/nvs-runtime/src/metrics.rs:105` gap 1 — stage 5, but its own file set.
- `crates/nvs-cli/src/cache.rs` has no test that places a Mach-O payload; `linkage_name` is asserted
  directly instead — `crates/nvs-cli/src/cache.rs`'s `mod tests`.
