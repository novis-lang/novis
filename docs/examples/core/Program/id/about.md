Returns a text that identifies this version of your program.

`Core\Program::id()` returns 64 lowercase hexadecimal characters. The value is a hash of all the code
in your program and of the environment it was compiled for. The same code on the same server gives
the same value on every run. When you deploy a change, the value changes too.

This makes it a good key for anything that must be thrown away after a deployment: a cache entry, a
version label in a log, or a header that tells a browser to load a file again.

**Good to know:** the value is safe to print. A hash does not show any of your source code.

**The examples below** print the length of the id, make a short version label from it, and build
cache keys that change with every deployment.
