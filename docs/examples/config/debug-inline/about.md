Decides whether a debugging dump also appears in the page, or only in the log.

`Core\Debug::dump` always writes what you hand it to the log. With this switched on, a dump made
while an HTML page is being built is appended to that page as well, where it is right in front of you
on your own machine — and in front of everyone else on a public one. A JSON response is never
modified either way, so an API answers the same shape in both.

Which way it goes is chosen by the mode a deployment runs in: development appends, production does
not. A program can never turn it on for itself, which is what makes a dump forgotten in shipped code
a line in the log rather than a page full of your own data.

**Good to know:** on the command line a dump always goes to the error stream instead, so this key
has nothing to decide there and your program's own output stays clean.
