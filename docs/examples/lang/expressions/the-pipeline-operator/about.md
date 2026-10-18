Passes a value through several steps, written from left to right in the order they run.

You write the value, then `|>`, then the step. Inside the step, `$_` is the place where the
value goes. Each step takes the result of the step before it. To trim a string and then convert it
to lower case, you write two steps.

Without `|>` you write one call inside another, and you read the steps from the inside to the
outside. Both forms do the same work, and `|>` adds no cost when the program runs.

**Good to know:** each step needs exactly one `$_`. A step with none, or with two, does not
compile. The right side of `|>` is the call itself, with `$_` in it.

**The examples below** clean up a form field in two steps, build a URL slug in five steps and
tidy a list of tags. Example 04 uses `|>` with other operators. Examples 05 and 06 do not compile.
