Returns the keys of the files on a disk, sorted.

The disk is a `[storage.<name>]` block in `nvs.toml`, and the `fs.read` capability must include its
folder. `get` can read every key in the result. With the `prefix` option, `list` returns only the
keys that start with that text, for example every key that starts with `invoice-`.

**Good to know:** only files with a valid key are in the result. A folder, a symlink and a file
whose name starts with a `.` are not. The keys are sorted by their bytes, so `B` comes before `a`.
