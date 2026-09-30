Passes a value through several steps, written from left to right in the order they run.

You write the value, then `|>`, then the step. Inside the step, `$_` marks the place where the
value goes. Each step takes the result of the step before it. To trim a string and then convert it
to lower case, you write two steps.

Without `|>` you write one call inside another, and you read the steps from the inside to the
outside. Both forms do the same work, and `|>` adds no cost when the program runs.

**Good to know:** each step needs exactly one `$_`. A step with none, or with two, does not
compile. In PHP 8.5 the right side of `|>` is a function. Here it is the call itself, with `$_` in
it.

**The examples below** clean up a form field in two steps, then build a URL slug in five steps,
then tidy a list of tags.
