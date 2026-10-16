A call your program is not allowed to make throws a `RuntimeError`.

Reading a file, writing a file and starting another script each need a capability. A capability is
permission to do one kind of thing. The person who runs the server grants it in `nvs.toml`, and
nothing is granted by default.

A call without the grant throws before it does any work. The message names the method, the capability
it needed and the path it was given. A `help:` line under it says where to write the grant. This is an
ordinary throw, so `catch (RuntimeError $denied)` takes it and the program keeps running.

Your program can use that. Try the call, catch the error, and do the work another way if it is not
allowed.

**The examples below** show a call nobody allowed, and a grant that lets one call through while a
second call is still denied. The third is a program that finishes its work without the capability.
