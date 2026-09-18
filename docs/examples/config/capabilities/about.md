Says what a program is allowed to do. Until a block here names something, the answer is no.

Every effect that leaves the program sits behind a grant: opening a file, dialling a host, running
another script, reaching a database, using the shared cache. A grant is either a plain yes or a list of
the exact places it covers — the directories a program may read, the hosts it may dial. Blocks layer
widest first, so a deployment can grant something to everything it runs and then take it back again for
one program.

**In plain words:** a key ring cut in advance. A program is handed exactly the keys the file cut for it,
and while it is running it can neither cut another one nor throw one away.

**Good to know:** a program can read its own grants back but can never change them, not even to give one
up, so there is only ever one place to look to see what it may do.
