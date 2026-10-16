Shows a numbered list of choices at the terminal and returns every choice that was chosen.

`Core\Cli::multiSelect` writes one line per choice, numbered from `1`, and then the question. The person
types the numbers they want, separated by commas or spaces. The call returns those choices themselves, not
their numbers.

The answer is a set. A choice named twice is in it once, and the order is the order of your list rather
than the order the numbers were typed in. An empty line is the empty set, which is how somebody chooses
none of them. A number that is not on the list is not an answer, so the question is asked again. `1-3` is
not a way to write a range, so it is asked again too.

There is no `default` option, because an empty answer already means "none of these". Where there is no
terminal, in a nightly job for example, the call throws `Core\Cli\NotInteractive` at once.

**The examples below** choose which features to switch on, put readable labels on a list of numbers, and
restart the services somebody picked.
