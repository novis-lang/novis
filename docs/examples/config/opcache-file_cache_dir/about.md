Where this deployment keeps the code it has already compiled.

A compile is expensive and its result is worth keeping, so Novis writes compiled units to disk and
maps them back on the next start. This key is where that directory is, and it is the only spelling of
it: `[cache]` is the data cache an application uses from its own code and holds nothing about
compiled units.

**In plain words:** this is where the bytes the process is about to execute come from.

Which is why no program may point it anywhere. Redirecting where already-compiled, about-to-be-trusted
native code is read from is a way to choose what gets executed, not a performance knob, and there is
no safer direction to move it in either — a request cannot narrow it any more than it can widen it.

It is also the one key in its block that waits for a restart. The others are read by the next
compile; the directory is what every unit this process has already mapped was read out of, so moving
it under a running server is not something a reload can do.

The example prints the file cache this checkout uses and is turned away trying to move it.
