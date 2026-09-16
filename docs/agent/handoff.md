# Handoff

## State

**Goal `unowned-closures`, stage 6 — the register.** `python tools/owners.py` reports `unowned: 32`,
`past-milestone: 8`, `untagged: 0`, `broken-tag: 0` and `unreasoned: 0`; the stage wants the first at 0,
and its other check, `python tools/owners.py --deferrals`, is green. `owners.py` § *UNOWNED* is the
roster of all 32, each with its reason in `docs/agent/carried-gaps.md` § *Unowned*.

**`nvs-types` is down to one unowned gap**, `intrinsics.rs:94` (gap 6), the item below. This session
struck three: `error_lib.rs`'s was stale (`?->` already reads straight off `$e->previous`, and what
rules out narrowing it in place is `rule:types/narrowing`'s subject, now stated there), `locals.rs`'s
was the exact join written up as a hole, and `lib.rs`'s was half stale and half built — `E0739` already
walked method bodies, and `E0822` now refuses a written `return;` under a non-`void` declaration, both
now read over a `get` hook's and a block-bodied `fn`'s body too. `python tools/verify.py` is green at
11 of 11 (conformance 2001). Nothing is blocked.

## Next group

**Stage 6: `nvs-types`' last unowned gap, then `nvs-ir`'s cluster** — one file set:
`crates/nvs-types/src/intrinsics.rs`, `crates/nvs-types/src/check.rs` and `crates/nvs-cli/src/`'s check
path. The first item is the same decision the last three were, taken per the goal's § *Standing
decisions*: build it, state it as a bound (which strikes the gap), or defer it to an M9+ milestone whose
plan states the scope. **Read the code before believing the gap** — every gap this session took
described a hole the tree had already closed or had never had, and `nvs.exe check` on a ten-line scratch
file under `.agent-tmp/` settled each one faster than the prose did.

- [ ] **`nvs check` either reads a configuration or the host check is scoped to the callers that do** —
      `crates/nvs-types/src/intrinsics.rs:94` (gap 6), `rule:core-classes/db-literal-query-checking`.
      `crate::Env::grants` is the channel and `crate::check::check_program_granted` fills it; `nvs-cli`'s
      check path hands over no `nvs.toml`, so the refusal fires for nobody. A command that reads
      configuration is a command a broken `nvs.toml` can fail, which is the decision.
- [ ] **A method that writes no return type is accepted, and `rule:types/declaration` says every one is
      written** — `crates/nvs-types/src/check.rs:678` (`lower_optional_type` answers `mixed` for an
      absent one) and `rule:types/declaration`. `public static function m() { … }` compiles today, which
      is why `E0739` and `E0822` both guard on `m.return_type.is_none()`. Either the slot is made
      mandatory where it is lowered, or the rule states the exception; this is a gap in the *rule's*
      coverage rather than a tagged one, so it has no owner line to strike.
- [ ] **`crates/nvs-ir/src/lib.rs`'s 11 unowned gaps are the largest cluster left** — `:209`, `:219`,
      `:261`, `:291`, `:302`, `:376`, `:421`, `:434`, `:470`, `:496`, `:532`, all under one
      `# Known gaps` block, with `docs/agent/carried-gaps.md:152` holding the one entry that reasons for
      all of them. Take them as one reading of that block rather than one gap at a time.

## Backlog

- `crates/nvs-cli/src/openapi.rs` gaps 1–5 and `crates/nvs-host/src/{group,placed,worker}.rs` are the
  other two clusters; `python tools/owners.py` § *UNOWNED* is the roster.
- The 8 § *DEFERRED TO A MILESTONE THE PROGRAM HAS ALREADY PASSED* items are owed by a goal or nobody
  and are not counted by this stage's check — `docs/agent/carried-gaps.md` § *Unowned* is their index.
- `docs/agent/goals/52-plan-truth.md:107` names `crates/nvs-types/src/lib.rs` gaps 1–3; that block is
  now gone entirely, so the line reads as a dossier of what was found rather than a live reference.
- `crates/nvs-types/src/check.rs`'s `check_body_exits` is the only reader of
  `crate::returns::for_each_valueless_return`; a second door added later wants the same call rather than
  a fourth walk.
