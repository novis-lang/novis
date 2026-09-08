# Handoff

## State

**Goal 16 — a body is read once, and JSON is one of the ways to read it. Stage 4 is closed**:
both members are whole, their five unit tests and six `tests/conformance/core/` cases pass, and
the qualifier refusal `rule:security/derived-codec-qualifiers` asks for now exists and is pinned.
Stage 1 is goal 15's whole list; **stage 5 is the only open one**, and it is what the driver's
acceptance check has been failing on: `examples/json-body.nvs` does not exist yet.

**`E0810` is the call-site refusal.** `crates/nvs-diagnostics/src/lib.rs:3248` declares it;
`crates/nvs-types/src/derive.rs:@check_decode_sites` is the pass, run from
`crates/nvs-types/src/check.rs:222` beside `check_row_sites` and for its reason — the class a
call writes is routinely declared in a file the walk has not reached. A site is recorded at
`crates/nvs-types/src/expr/args.rs:1487`, which is the one place a written class and the member
that asked for it are both in hand.

**What it walks, which is wider than the written class.** Every `#[Json\Derive]` field the
document reaches: the written class's own, and those of every deriving class its fields name,
with an array unwrapped to its element type. One report per property however many sites decode
into it, because the fix is one `tainted` on one declaration. `secret` never reaches it —
`derive.rs`'s `codec_field` refuses a `secret` property where it is declared.

**`Core\Json::decodeAs` deliberately gets no site**: its `$json` parameter is a plain `string`,
so a tainted document is already `E0401` where it is passed. `args.rs:1474`'s comment is the home
of that split.

## Next group

**Stage 5: the proofs — the runnable fixture** — one file set: `examples/`,
`crates/nvs-test/src/request.rs`, `tools/reference.py`.

- [ ] **`examples/json-body.nvs`, printing `docs/agent/loop-goal.toml:5521`'s six `want` lines
      in order** — the leg runs it under `nvs run --request`, so a body's own facts are
      assertable here rather than narrated. `crates/nvs-test/src/request.rs:1` owns the request
      file's format and `docs/agent/loop-goal.toml:5515` is the check itself; the four `Core`
      calls it needs are `json`, `jsonAs`, `method` and `body`, all landed.
- [ ] **The two `ParseError` lines and the content-type one are the same program's `catch`
      arms** — an absent and an empty body are both `ParseError` and a mislabelled but valid
      document is read, per the goal's § *Standing decisions*;
      `tests/conformance/core/a-malformed-json-body-is-a-parse-error.nvst:1` already pins the
      first of the three and is the shape the fixture prints around.
- [ ] **Regenerate the one-file reference so `json` and `jsonAs`'s cards are in it** —
      `python tools/reference.py --check` is `docs/agent/loop-goal.toml:5532`'s check and
      `tools/reference.py:708` is the generator; it is generated from `nvs meta --json` and never
      edited, so a stale card fails there rather than in `nvs-stdlib`.

## Backlog

- `rule:security/derived-codec-qualifiers`' declaration half (a `secret` property on a deriving
  class) is landed in `derive.rs`; nothing else of the rule is open.
- A `Core\Db\…::queryAs<T>` gets no qualifier pass: a row is not a peer's document, and
  `derive.rs:@db_reachable`'s comment says § 6 makes every text column tainted on the way out.
- `docs/agent/carried-gaps.md` owns anything that must outlive this goal.
