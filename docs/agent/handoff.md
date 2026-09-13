# Handoff

## State

**Goal `plan-truth` is through stages 2, 3 and 4.** Stage 4's check — `python tools/owners.py` — reports
`0 owned by a retired goal`, with `0 untagged, 0 tagged wrongly, 0 unowned with no reason` still beside
it. Five module-doc gaps named a goal that had walked without closing them: two are struck because the
code closed them, three are re-owned. The three stale sentences the previous group named are rewritten.

Struck, each verified in the source first: `crates/nvs-cli/src/serve.rs` — a served request *does* carry
the configuration (`crates/nvs-server/src/serve.rs:1101` writes the snapshot, `ctx/wiring.rs:366`
`set_config` calls `refresh_limits`, `ctx/limits.rs:407` returns the ceiling); `crates/nvs-types/src/lib.rs`
— `is` *does* narrow (`crates/nvs-types/src/locals.rs:409` dispatches `type_test_residue` at `:557`).
Re-owned: LSP completion's inherited-member and visibility gaps to **M10**, whose plan takes `nvs-lsp`
past M4B's minimal slice (`docs/plan/m10.md:12`, `:47`, `:58`); the router's mount prefix to
**`m7-server-surface`**, which promises linking through the mount; `Core\Queue`'s `limits`/`grants` to
**`gap-zero`**, beside the same file's gap 4.

Stages 1 and 5 are open. Stage 1 is goal `websocket-client`'s list, carried as the floor.

## Next group

**Stage 5: the index** — one file set: `docs/agent/carried-gaps.md`'s § *Owned* table. The check is
`python tools/playbook.py --check`, wanting `none -- every carried-gaps owner is live or struck`; fifteen
rows, `:48`–`:64`, name a retired goal. This file's § *The contract* is the rule for all four items: a row
leaves only when the gap is closed, so an owner that went green is **struck**, never renamed to hide it.

- [ ] **The row the code already closed is struck, not re-owned** — `docs/agent/carried-gaps.md:57` says
      "`nvs serve` runs on one core, and no path in the process starts a second". It does not: the fleet
      loop at `crates/nvs-cli/src/serve.rs:427` starts one worker per core over `cpus[index % cpus.len()]`,
      which this session read. `rule:http-server/the-accept-fan-out-is-one-worker-per-core` is the rule.
- [ ] **The two rows this session already answered in the module doc** —
      `docs/agent/carried-gaps.md:61` takes owner `gap-zero`, to match `crates/nvs-stdlib/src/queue.rs:85`;
      `docs/agent/carried-gaps.md:62` takes `unowned-closures`, to match
      `crates/nvs-stdlib/src/lib.rs:155`, where row 62's own *decided: widen to a covariant read* is now
      written as that gap's `Decided:` sentence. `rule:types/arrays` is the invariance the widening amends.
- [ ] **The five rows owned by the retired goal `carried-gaps`** — `docs/agent/carried-gaps.md:48`, `:49`,
      `:51`, `:52`, `:54`. Each one's module doc already names a live owner (`owners.py` is green), so the
      row follows the module doc rather than deciding for itself.
- [ ] **The remaining rows** — `docs/agent/carried-gaps.md:55`, `:56`, `:58`, `:59`, `:60`, `:63`, `:64`.
      Owners `test-request`, `per-core`, `net-os-signal`, `gap-owners`, `unowned-sweep`, `input-shapes`
      and `config-is-written` have all walked.

## Backlog

- `nvs_types::expr::is_assignable`'s own docs are cited as saying no variance was committed to, while
  `rule:types/arrays` states invariance flatly — same stage-4 pass, and `owners.py` does not see it
  because it is not a `# Known gaps` item. `crates/nvs-types/src/expr/`.
- `crates/nvs-test/src/case.rs:351`'s `NOT_YET` reason string still names M6; correcting it is a code
  change, which this goal's § *Standing decisions* forbids.
- `crates/nvs-types/src/lib.rs:156` says definite assignment is "no longer conservative" — changelog
  wording a comment may not carry (`AGENTS.md` rule 6).
- Stage 1 is goal `websocket-client`'s carried floor and nothing in it is known red.
