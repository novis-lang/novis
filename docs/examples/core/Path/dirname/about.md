Returns the folder that contains a path. This replaces PHP's `dirname`.

The method reads the path as text and never looks at the disk. It accepts `/` and `\` as separators
on every platform. The result is written with `Core\Path::SEPARATOR`, which is `\` on Windows and
`/` everywhere else.

The option `levels` says how many parts to remove from the end. The default is `1`. With `levels: 2`,
`/srv/shop/config/app.toml` gives `/srv/shop`. When there is nothing left to remove, the result is
the root for an absolute path and `.` for a relative one. So the result is always a folder you can
use.

**Good to know:** `dirname("")` is `.`. PHP returns the empty string. `levels: 0` returns the path
with its separators cleaned up. PHP throws an error.

**The examples below** find the folder of a file, go up several levels, and find a folder next to a
configuration file.
