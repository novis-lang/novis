`Core\Script::args()` returns the value that this script was started with. One script starts another
with `spawn script`, and it can give the new script a value with `args:`. The new script calls
`Core\Script::args()` to read that value. It receives its own copy, so a change in one script does not
change the other.

The value can be anything that `args:` accepts: a string, a number, an array or a nested array. Its
type is `mixed`, so you convert it before you use it. When no value was given, the result is `null`.
This is also the result in the first script, because no other script started it.

**The examples below** show a script that greets the name it was given, a script that uses a default
when it was given nothing, and a list of orders split between three scripts that run at the same time.
