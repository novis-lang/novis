Returns when a file was last changed.

`$info->modifiedAt()` works on the `Core\IO\Metadata` value that `Core\IO::stat` returns. It
returns a `Core\Time\Instant`: the moment the file was last written, as the operating system
recorded it. A folder has a time too.

The value is the time from the moment `stat` was called. If the file is written again later,
this time stays the same. Call `stat` again to get the new time.

A `Core\Time\Instant` has no time zone. You can compare two of them with `compareTo`, and
`Core\Time::now()->since(...)` returns how long ago the file was changed.
