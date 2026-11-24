Keeps the temporary directories of a program after it ends. Without this setting, they are deleted.

A program gets its own directory for working files with `Core\IO::createTemporaryDirectory`. Novis deletes that directory when the program
ends. This happens when the program ends normally, stops early or stops with an error. With this
setting on, Novis deletes nothing and writes each kept directory to the log. You can then look at the
files that a failing run left. Turn the setting off again afterwards, because nothing else deletes
those directories.

**Good to know:** only the person who runs Novis can set it, in `nvs.toml`. A program cannot keep its
own files this way. If it could, an attacker could make a program fill the disk.
