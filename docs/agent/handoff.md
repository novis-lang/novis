# Handoff

## State

**Goal 4 of the parity program has just started; nothing of it has landed yet.** M4 and goals 1–3 reached
their whole acceptance lists and all four are now this goal's Stage 1 floor.

**This is the largest goal in the program** — nine subsystems, each with its own ADR — and it is
deliberately not cut smaller. Its ten stages are grouped by *file set*, so take a group from one stage and
stay in it; the goal's size is meant to cost sessions rather than to cost any one session its context.

Everything this goal needs is already built: goal 2's parking stream is what every network member is
written over, goal 2's blocking pool is where every call with no readiness goes, goal 2's graph copy is
what `Core\Cache` copies with, and goal 3's capability gate is what every member here declares against.
**None of those is re-invented**; a session that finds itself writing a second one has taken a wrong turn.

## Next group

**ADR 0056's regex tiering** — Stage 0, and it is a catch-up rather than a feature. Goal 1 shipped
`Core\Regex` against one engine and folded its literal patterns; this changes what an existing pattern
*does*, so every case written before it lands is written against the wrong engine.

One file: `crates/nvs-stdlib/src/regex.rs`, plus the checker half in `crates/nvs-types`.

- [ ] **A linear-time engine by default, backtracking only for what it cannot express.** ADR 0056. The
      two crates the user named — `regex` and `fancy-regex` — are exactly this pair, and they are already
      in `[workspace.dependencies]`. What is missing is the *rule*: which patterns route where, and the
      fact that routing is not a performance heuristic but a semantic guarantee.
- [ ] **Backtracking runs under a throwing step budget.** A budget exhaustion **throws**; it does not
      return "no match". Returning no-match turns a denial-of-service into a silent authorization bypass
      wherever the pattern was a check, which is why the ADR is emphatic about it.
- [ ] **A literal pattern's tier is settled at compile time**, and the pattern argument refuses `tainted`.
      Goal 1's folding pass is where the first half hangs — it already prepares a literal pattern, so this
      is that pass recording which engine prepared it, not a second pass.

## Backlog

- Stage 6's shared tier is Redis (ADR 0059 § 1). The container it runs in is the same compose file goal 5
  uses; if it is not up yet, `Core\Cache::local()` and `shed` are the halves that need no store and are a
  legitimate slice on their own.
- `Core\Session`, `Core\Metrics`, `Core\Router::match` and `Core\Queue` are **not** in this goal — the
  first three need a request (goal 6), the last needs a database (goal 5).
- PHP's optional extensions — `gd`, `intl`, `imap`, `zip` — are M9's and are not parity work.
