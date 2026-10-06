Returns the time a file was last changed.

`Core\IO::modifiedAt` returns the time the file at the path was last written, as a
`Core\Time\Instant`. An instant is one point in time. It has no time zone, so it means the same
moment everywhere. Use `->in($zone)` to show it as a date and a time of day.

You can compare two instants with `compareTo`, and measure the time between them with `since`. This
is how a program finds out whether one file is newer than another, or how old a file is.

A folder has a modification time too. When there is nothing at the path, `modifiedAt` throws an
`IOError`.

The program needs the `fs.read` capability for the path. Without it, `modifiedAt` throws a
`RuntimeError`.
