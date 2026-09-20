Joins several `bytes` values into one.

You give the list of buffers, and a separator when you want one. The result is a new value holding
every buffer in the order of the list. The separator is written between neighbours only, so there is
none before the first buffer and none after the last one. Without a separator the buffers follow
each other directly.

A list with one buffer returns that buffer's content, and an empty list returns a value with nothing
in it. A buffer with nothing in it still counts as one part, so the separators around it are
written. Nothing is converted here: every element is already a `bytes` value.

**Good to know:** this is how a program puts the chunks of a body back together, or builds one
message out of the pieces it collected.
