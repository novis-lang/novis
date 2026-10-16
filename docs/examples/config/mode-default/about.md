The mode this host runs in.

Novis has two modes and no third: production, and development, which trades the safe defaults away
for the detail a developer wants. This key is where that choice is written, and where a program reads
it back. Written nowhere it is production — a deployment that forgets the line is the safe one, and
the developer who wants the other one asks for it once.

The mode is a shorthand, not a switch: it changes the default of five ordinary directives — how logs
are formatted, how much of them is kept, whether requests are logged, how much of an error a caller
sees, and whether a dump is printed inline. Each of those is still spellable on its own line, so a
mode beside an explicit value means what it reads like.

A program may flip its own mode, as far as `[mode] ceiling` allows. A name that is neither mode is
refused; staging is a production host with different configuration, which is what staging has always
been.

The example reads the mode, branches on it the way an application does, and is turned away asking for
a third one.
