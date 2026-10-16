How many old log files Novis keeps next to a log file.

This setting is used when `[log] target` is a file. When the file reaches `[log] max_size`, Novis
renames it to `nvs.log.1`, and the old files move up one number. A file whose number would be
larger than `keep` is deleted. With `keep = 5` and `max_size = "10M"`, the log uses at most 60 MB
of disk: the current file and five old ones.

The value is a whole number. When it is not set, it is `5`. With `keep = 0`, Novis deletes the
full file and starts a new one, so no old records stay on disk. `false` and negative numbers are
not allowed, and the server does not start.

Only the person who runs the server can change this setting. A program cannot change it.

The example prints the setting and shows that a program cannot change it.
