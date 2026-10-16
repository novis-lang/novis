The door's CSRF key written as a path instead of a value: the file's content is the key.

This is the spelling a deployment uses when something else issues the key — a secret manager, or an
orchestrator mounting a file into the container — so the key never sits in a configuration file that
gets committed and shared. The file's whole content is the value, with one trailing newline removed
and nothing else trimmed, so a key written by one tool is the same key when another tool wrote it.
A program then asks for `csrf_key` and receives the content; nothing asks for the path.

Exactly one of the pair may be written. Both together is refused when the server starts, because one
value with two sources is a question about which wins that nobody should have to ask. The file is
checked like every other configuration input: an account other than the server's that can rewrite it
would be choosing the key, so the server refuses to start rather than believing it. A file that is
merely readable by others is warned about and read.
