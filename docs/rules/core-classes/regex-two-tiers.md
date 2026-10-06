Every pattern compiles to the **linear tier** if the linear engine can express it, and to the
**backtracking tier** otherwise. The choice is made by the engine, never by the developer and never
by a modifier: there is no way to ask for backtracking, only to write a pattern that requires it.

The backtracking tier runs under a bounded step count. Exhausting it throws an ordinary catchable
`Throwable` naming the pattern and the budget. It never returns "no match", never returns a falsy
value, and never truncates the search — a search that stopped early and a search that found nothing
are different facts, and one falsy return would conflate them. Per
`rule:errors/escalation-ladder` this is an ordinary throw rather than a resource-limit fatal, so the
request may catch it and answer 400. The linear tier has no budget, because it needs none.

What this costs is that a pattern's performance class is a property of the pattern rather than
something a caller can override. That is the trade taken deliberately: an engine choice a developer
cannot see is the failure mode the whole design exists to avoid.

The budget's default is `[limits] max_regex_steps`, an ordinary `Runtime` directive
(`rule:config/three-changeability-classes`): a request may widen or narrow it for itself, a
`[limits.hard]` entry is how a host bounds that, and a deployment that has written nothing gets the
constant `crates/nvs-stdlib/src/regex.rs` states. `false` does not spell an unbounded tier — it
reads as that same constant, because a pattern allowed to backtrack forever is the hang this rule
exists to stop.
