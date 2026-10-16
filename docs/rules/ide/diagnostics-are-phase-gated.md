The front end runs parse → declarations → resolution → types with no gate between the phases, bailing
only after the type check. In batch mode that is right: you read the first error and the process exits. In
an editor it is not — resolution running over an `ExprKind::Error` node reports `E0301` ("assigned to but
never declared") *above* the `E0102` that caused it, and on every keystroke mid-statement that is a wall of
red whose topmost entry is wrong, which teaches a developer to stop reading the squiggles.

So, expressible because the code bands are allocated by phase: **a file that has produced a lexer
(`E00xx`) or parser (`E01xx`) diagnostic publishes those and its declaration diagnostics, and suppresses
resolution (`E03xx`) and type (`E04xx`) diagnostics for that file only.** Not for the workspace, and not
for the phases below the failure: the other files in the graph keep their own diagnostics, because a
broken buffer in one tab is not a reason to go dark in another.

This is presentation, not analysis. The checker still runs and `nvs check` is untouched, so no diagnostic
is lost anywhere one was reaching a human before. The suppression is one-directional — a resolution error
never suppresses a type error, because those two do not cascade the way a parse failure into everything
below it does. A `.lspt` case pins it in both directions, with `phase=all` defeating the gate.
