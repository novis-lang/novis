# Handoff

## State

Goal `lang-expressions` is **complete and re-confirmed**. All sixteen features carry their feature
proofs; `python tools/dossier.py --verify --group lang:expressions` prints `nothing owed` with both
suites at `0 failed`, `python tools/verify.py` is 14 of 14 green, `python tools/verify.py --doc`
resolves every link, and `owners.py --closes lang-expressions` and `playbook.py --closes
lang-expressions` each own nothing.

The acceptance failure that held the previous DONE was real when the sweep read it and is already
fixed — see the new playbook bullet. Nothing in the tree was owed.

Two bench findings stay recorded rather than fixed
(`rule:testing/a-failing-proof-is-fixed-or-recorded`'s second answer), as `# Known gaps` 10 and 11 in
`crates/nvs-runtime/src/lib.rs`: a call through a `callable` builds its argument slots in a heap
vector, 32 bytes a call, and it reaches the callee without passing `nvs_probe_call_enter`. Both
benches carry the `dossier: known-gap` marker naming that file.

**A second writer holds this tree.** `tools/verify.py` (a per-step green cache and a new `nvs fmt`
step) and `docs/agent/commands.md` are uncommitted and are not the loop's; `tools/verify_keys.py` is
new and untracked. Verification is green under all three, but stage nothing you did not write.

## Next group

The chain's next goal is `lang-statements`, whose features owe the same feature proofs each. One
slice is one feature with all its feature proofs, and they share one file set:
`docs/reference/lang/40-statements.md` plus the proof trees under `docs/examples/lang/statements/`,
`tests/hostile/lang/statements/`, `benches/members/lang/statements/` and `tests/conformance/`.

**Stage 2: the dossier** — one file set, named above.

- [ ] **`lang:statements/expression-statements-blocks-and-declarations`** — owes all five. What is a
      statement rather than an expression, what a block scopes, and where a declaration may stand.
      `rule:testing/feature-proofs`. `docs/reference/lang/40-statements.md:9`
- [ ] **`lang:statements/if-elseif-else`** — owes all five. The condition resolves
      `rule:expressions/truthy-table`, `elseif` is one word, and only the arm that is taken runs.
      `docs/reference/lang/40-statements.md:44`

## Backlog

- A call through a `callable` answers `mixed` and `mixed as callable` is refused, so a curried
  closure cannot be stored: `$f(5)(2)` works inline but `callable $g = $f(5)` does not
  (`rule:types/callable-is-a-closure`).
- `lang:expressions/and-the-ternary` and `lang:types/widening-without-as` still carry the two example
  known gaps recorded in `crates/nvs-ir/src/lib.rs`.
- `lang:expressions/assignment`'s bench declares `allocations 0` and does four; it is marked, and the
  entry it names is that crate's to close.
- The driver's context-sync credited a by-hand commit's paths to the session and added
  `crates/nvs-stdlib/src/jwe.rs` to goal `lang-expressions`'s `[context] modules`; it was reverted
  unstaged, and `tools/context-sync.py` is where a filter by author would go.
