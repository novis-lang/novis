Saves bytes as a file with a name, on a disk that `nvs.toml` sets up.

A disk is a `[storage.<name>]` block in `nvs.toml`. Its `root` is the folder that holds the files,
and the `fs.write` capability must include that folder. The key is the name of the file. It can use
ASCII letters, digits, `.`, `-` and `_`, and it is at most 255 bytes long. A key with a `/` or a `\`
throws an error, so a program cannot write outside the folder.

If a file already has the key, `put` replaces it. With `overwrite: false`, `put` saves the file only
when the key is free. Otherwise it throws an `IOError`.

**Good to know:** the key may come from user input, but the disk name may not. You write the disk
name in the code.
