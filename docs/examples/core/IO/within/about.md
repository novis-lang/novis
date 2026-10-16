Checks that a file name stays inside one folder, and returns its full path.

`Core\IO::within($base, $path)` joins `$path` to the folder `$base` and resolves the result. It
follows every `..` and every symbolic link. If the final path is inside `$base`, the method
returns it. If it is outside, the method throws a `RuntimeError`. The file itself does not need
to exist yet, so you can check a name before you create the file.

Use it when a file name comes from a user, for example from a form or a URL. A name like
`../../config.toml` then cannot reach a file outside the folder. The returned path is safe to
pass to `Core\IO::readText`, `Core\IO::write` and the other file methods.

The program needs the `fs.read` capability for both paths. `within` throws an `IOError` when
`$base` does not exist.

**In plain words:** it is a guard at the door of one folder. A name that leads out of the folder
does not get past it.
