With no controlling terminal, a prompt returns its `default` if one was given and otherwise throws
`Cli\NotInteractive`. **It never blocks.** A CLI that hangs in CI waiting for an answer nobody can
give is the same failure as an outbound request with no deadline, and there is no spelling for an
unbounded wait.

A terminal that is there and unattended is the same rule, so the read is under a deadline too — five
minutes, after which the prompt takes the same `default` or throws the same class with a sentence
naming the deadline rather than a missing device. A CI job that allocated a pty, or a `docker run -t`
with no keyboard behind it, has a terminal by every test a process can make, and a blocking read there
would hang as long as the pipeline's own timeout allows. **No parameter lengthens it**, on any of the
five members: a `timeout` option would be the spelling for an unbounded wait, and a person who walks
away mid-answer getting a throw is priced at `rule:programs/memory-priority`'s ordering, where a
program that cannot hang outranks one that never gives up.

Under `nvs test`, prompts drain a scripted answer queue instead of reading a terminal:
`Core\Test::scriptAnswers(array<string> $answers)` — a member on the class the *test* calls, never a
filler on `Core\Cli` that would let production code answer its own prompts. It is drained where all
five prompts already meet, so a sixth inherits it. A scripted line wins over a terminal that is there,
and a queue with nothing left is the unattended run above, so scripting too few answers is assertable
rather than a hang.
