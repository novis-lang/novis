- **A state-bleed suite parameterised over a request boundary and an isolate boundary cannot have a
  memory row.** "The next run's arena is its own" holds across a request but not an isolate, because
  `rule:security/isolate-shares-nothing` gives a child isolate its parent's budget. Parameterise
  only the state a rule says is never shared and keep memory in its own cases: only
  `nvs_runtime::budget::live_bytes()` sees what an earlier run left. [until: gone crates/nvs-runtime/src/budget.rs:live_bytes]
