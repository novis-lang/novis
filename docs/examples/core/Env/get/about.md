Returns the value of one environment variable, or `null` if nothing set it.

The value is tainted: it comes from outside your program, so you must check or escape it before you
use it in a query, a URL or a command. The name must be text your program wrote itself. A name that
came from a request does not compile, because a visitor could then read every variable, one request
at a time. An empty value is still a value, so `null` means only one thing: the variable is not set.

**In plain words:** tainted means "not checked yet". You can compare the value or write it to a
log. You cannot put it into a query or a command until you have checked it.

**Good to know:** if the value is not valid UTF-8 text, `get` throws a `RuntimeError`.
