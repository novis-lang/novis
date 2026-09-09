# Handoff

## State

**Goal 38 — an encoder ends a cycle where it closes. Stage 2 has landed: `Core\Json::encode` carries
the ancestor chain, refuses at the first repeat, and names the dotted chain of keys that closed it.**
Goal 25's whole list remains this goal's stage 1 floor and is untouched.

The design is settled by [0164](../decisions/0164.md) and its § *Standing decisions* — the set is the
ancestor chain, the answer is a throw with no marker in the document, the depth cap stays. Nothing
there is a session's to re-decide, and stage 2 decided nothing.

One fact 0164 could not know, now measured and recorded as `crates/nvs-stdlib/src/json.rs`'s known
gap 7: the depth ceiling was never reachable on the encode side. A self-referencing object exhausted
the native stack and **aborted the process** — it was never reported as nesting, because nothing was
ever reported. So this stage removed a crash rather than a misdiagnosis, and what is left behind the
gap is an acyclic document deeper than the stack, which nothing schedules yet.

## Next group

**Stage 3: the other walkers, audited rather than assumed** — one file set:
`crates/nvs-stdlib/src/csv.rs`, `uri.rs` and `encoding.rs`. Each answers one question — can this walk
reach an object graph at all — and the answer is written down either way, because "we checked and it
cannot" is the finding that stops the next reader checking again. A walker that **can** takes stage
2's treatment and its own case; one that **cannot** gets one sentence in its module doc and nothing
else. `Core\Serialize` is exempt by `rule:classes/graph-copy` and is not in this stage.

- [ ] **`Core\Csv::format`'s row walk, audited** — `crates/nvs-stdlib/src/csv.rs:433`, where the
      `Tag::Array` refusal already names what a row may hold, and the scalar readers at
      `crates/nvs-stdlib/src/csv.rs:335`. `rule:classes/an-encoder-ends-a-cycle-by-identity`.
- [ ] **`Core\Uri`'s query builder, audited** — `crates/nvs-stdlib/src/uri.rs:2396` and the scalar
      gate it walks values through at `crates/nvs-stdlib/src/uri.rs:1770`. Same rule; a walk that
      accepts only scalars per entry cannot reach a graph, and that is the sentence to write.
- [ ] **`Core\Encoding`'s argument walk, audited** — `crates/nvs-stdlib/src/encoding.rs:747`, whose
      arguments are `bytes` and `string` at `crates/nvs-stdlib/src/encoding.rs:667`. Same rule.

## Backlog

- Stage 4: `rule:classes/an-encoder-ends-a-cycle-by-identity` moves `designed` → `shipped` and its
  `guardedBy` names the landed cases — `docs/agent/loop-goal.md` § *Stage 4*.
- The encoder's real bound is the native stack, not `DEPTH_CEILING` — `crates/nvs-stdlib/src/json.rs`
  known gap 7. Nothing schedules it; a resource ceiling is goal 36's subject.
- `Core\Json`'s own gaps 1–6 are unchanged — same module doc.
