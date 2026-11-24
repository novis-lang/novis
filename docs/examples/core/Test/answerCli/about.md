`Core\Test::answerCli()` gives the answers for the next questions your program asks with
`Core\Cli`. It lets you test a command-line program that asks questions, with no person at the
keyboard. You pass an array of strings, one answer per question, in the order the program asks.

A question that `Core\Cli` asks is called a prompt. Each prompt takes the first answer that is
still waiting. It does not read the terminal. Write what
a person would type. `Core\Cli::confirm` reads `y` or `n`, and `Core\Cli::select` reads the number
of a choice. When an answer is not valid, the prompt asks again and takes the next answer.

A second call adds its answers after the ones still waiting. When no answer is left, a prompt
returns its `default`. A prompt with no `default` throws `Core\Cli\NotInteractive`.

You can call `answerCli` in a test and in a command-line program. In server code, it
throws a `LogicError`. Server code is a request, a connection, a scheduled task or a queued job.
There is one exception: a request that a test sends with `Core\Test::request` may call it.

**The examples below** show one answer, several prompts in a row, and a test of a setup program.
