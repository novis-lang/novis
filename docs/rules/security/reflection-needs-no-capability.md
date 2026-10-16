Reflection and source parsing are pure in-memory operations over a program's own compiled shape or its
own supplied string. Neither touches the filesystem, the network or another process, so neither sits
behind a capability grant the way a file read or process execution does: parsing a string is exactly
as ambient-authority-free as decoding JSON.

That is only true because a parsed tree is **inert typed data with no path back into execution**.
Reflection that could construct and run code would be an `eval` wearing a different name, and it would
need a grant precisely because it would then be an effect. The two rules hold each other up: the door
is free to be open because there is nothing behind it
(`rule:security/no-eval`, `rule:security/reflection-enforces-visibility`).
