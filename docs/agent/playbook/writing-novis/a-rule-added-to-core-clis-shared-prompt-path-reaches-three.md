- **A rule added to `Core\Cli`'s shared prompt path reaches three of the five prompts, not five.**
  `ask`, `confirm` and `secret` decide nothing before `ask_terminal`, but `select` and `multiSelect`
  ask `watched(ctx)` first and return early, which a grep for `ask_terminal` counting five call
  sites cannot see. Put anything that must reach every prompt in a predicate beside `watched` —
  `answerable` is the shape — that the early-deciding members read too. [until: gone crates/nvs-stdlib/src/cli.rs:ask_terminal]
