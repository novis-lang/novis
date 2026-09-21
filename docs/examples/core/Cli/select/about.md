Shows a numbered list of choices at the terminal and returns the one that was chosen.

`Core\Cli::select` writes one line per choice, numbered from `1`, and then the question. The person types
a number. The call returns the choice itself, not its number, so the value goes straight into the rest of
your program. A number that is not on the list is not an answer, so the question is asked again. A list
with no choices in it throws a `LogicError`.

The `labels` option is a function that turns one choice into the line the person reads. A list of numbers
or of objects needs it. The `default` option is the choice the `Enter` key takes. Where there is no
terminal, in a nightly job for example, the `default` is the answer. Give none and the call throws
`Core\Cli\NotInteractive` at once.

**The examples below** choose a region to deploy to, put readable labels on a list of numbers, and read
the settings of the environment somebody picked.
