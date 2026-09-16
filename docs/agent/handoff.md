# Handoff

## State

**Goal `unowned-closures`, stage 6 — the register.** `python tools/owners.py` reports `unowned: 31`,
`past-milestone: 8`, `untagged: 0`, `broken-tag: 0` and `unreasoned: 0`; the stage wants the first at
0, and its other check, `python tools/owners.py --deferrals`, is green. `owners.py` § *UNOWNED* is
the roster of all 31, each with its reason in `docs/agent/carried-gaps.md` § *Unowned*.

**`nvs-types` now has no unowned gap at all.** `intrinsics.rs` gap 6 was stale: `nvs check` has read
the tree since `nvs-cli`'s `front_end_granted` was built, and `crates/nvs-cli/tests/check_grants.rs`
pins all three halves of it — an ungranted literal host is `E0618`, no configuration is silence, and
a broken `nvs.toml` outranks both. What the pass does now is stated in its own module doc, and the
carried-gaps bullet whose `[until:]` condition that met is gone.

**`rule:types/declaration`'s return slot is now enforced** — `E0823`, at a method, an abstract
method and an interface member alike. The corpus was already unanimous (1596 non-constructor
signatures wrote the slot; the only one that did not is the `__construct` reject fixture, which
`E0114` already refuses and this check is deliberately silent on), so nothing in the trees moved.
The rule now states the constructor exception it always relied on. Conformance is 2003, differential
281, `python tools/verify.py` green. Nothing is blocked.

## Next group

**Stage 6: `nvs-ir`'s unowned cluster, the largest one left** — one file set:
`crates/nvs-ir/src/lib.rs`'s single `# Known gaps` block and `docs/agent/carried-gaps.md`. Read the
block whole before deciding anything: one carried-gaps entry reasons for all eleven gaps at once, so
this is likely one decision rather than eleven. **Read the code before believing the gap** — three
of the last four gaps taken described a hole the tree had already closed, and a ten-line scratch
file under `.agent-tmp/` run through `target/debug/nvs.exe` settled each faster than the prose did.

- [ ] **Decide `nvs-ir`'s eleven unowned gaps as one reading of one block** —
      `crates/nvs-ir/src/lib.rs:209` through `crates/nvs-ir/src/lib.rs:532` (gaps 2, 3, 4, 5, 6, 9,
      15, 16, 17, 19, 20), `rule:types/arithmetic` and `rule:expressions/one-equality-operator` for
      the operator rows among them. Per the goal's § *Standing decisions*: build it, state it as a
      bound, or defer it to an M9+ milestone whose plan states the scope.
- [ ] **Strike or re-own the one carried-gaps entry that reasons for the whole block** —
      `docs/agent/carried-gaps.md:152`, whose `[until:]` trailer names the tree state that retires
      it. It cites `docs/agent/carried-refusals.md`'s item 901, which is the argument it leans on.

## Backlog

- `crates/nvs-ir/src/lib.rs`'s remaining gaps after the cluster, if the block splits — its own
  module doc.
- `crates/nvs-cli/src/openapi.rs`'s five unowned gaps, the next cluster by size — that module's doc.
- `crates/nvs-host/`'s three unowned gaps across `group.rs`, `placed.rs` and `worker.rs` — one
  placement question in three files.
- The eight `past-milestone: 8` deferrals, which name a milestone the program has already passed and
  so are owed by a goal or nobody — `python tools/owners.py` names each.
