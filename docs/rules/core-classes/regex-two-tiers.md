Every pattern compiles to the **linear tier** if the linear engine can express it, and to the
**backtracking tier** otherwise. The choice is made by the engine, never by the developer and never
by a modifier: there is no way to ask for backtracking, only to write a pattern that requires it.

The backtracking tier runs under a bounded step count. Exhausting it throws an ordinary catchable
`Throwable` naming the pattern and the budget. It never returns "no match", never returns a falsy
value, and never truncates the search — a search that stopped early and a search that found nothing
are different facts, and one falsy return would conflate them. Per
`rule:errors/escalation-ladder` this is an ordinary throw rather than a resource-limit fatal, so the
request may catch it and answer 400. The linear tier has no budget, because it needs none.

A backtracking engine run against a crafted subject is exponential: `(a+)+$` can pin a core for
minutes. Under thread-per-core that is not one slow request but a lost core, and availability is
part of request isolation. A step limit whose exhaustion reads as "no match" turns that hang into a
wrong answer, which for an authorization or validation pattern is worse than the hang. The linear
engine removes the class outright at the call sites it covers, which are most of them, and it is
typically the faster engine there too.

What this costs is that a pattern's performance class is a property of the pattern rather than
something a caller can override. That is the trade taken deliberately: an engine choice a developer
cannot see is the failure mode the whole design exists to avoid.

The budget's default is `[limits] max_regex_steps`, an ordinary `Runtime` directive
(`rule:config/three-changeability-classes`): a request may widen or narrow it for itself, a
`[limits.hard]` entry is how a host bounds that, and a deployment that has written nothing gets the
constant `crates/nvs-stdlib/src/regex.rs` states. `false` does not spell an unbounded tier — it
reads as that same constant, because a pattern allowed to backtrack forever is the hang this rule
exists to stop.

**Declined:**
- **One backtracking engine for every pattern.** Perfect compatibility and no tiers to explain, but
  it keeps the exponential hang as a live risk in a runtime where one request holds a core.
- **The linear engine only, with lookaround and backreferences refused.** The strongest guarantee,
  but those constructs are common in real validation patterns, and refusing them pushes developers
  to hand-rolled parsing that is more likely to be wrong than the pattern was.
- **A time limit instead of a step limit.** Easier to configure, but it makes the outcome depend on
  machine speed and load, so the same program is deterministic on one host and not on another.
- **A per-pattern modifier that opts into backtracking.** The engine already knows the tier, the
  developer would have to keep the annotation in step with the pattern, and a wrong annotation is
  worse than none.
