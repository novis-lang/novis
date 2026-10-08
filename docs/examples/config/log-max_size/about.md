How large a log file can grow before Novis starts a new one.

This setting is used when `[log] target` is a file, such as `target = "file:/var/log/nvs.log"`.
When the next record would make the file larger than `max_size`, Novis renames the file to
`nvs.log.1`. Then it writes the record to a new, empty `nvs.log`. Older files move up one number,
so `nvs.log.1` becomes `nvs.log.2`. `[log] keep` sets how many of these old files stay on disk.

The value is a size, such as `"10M"` or `"1G"`. When it is not set, it is `"10M"`. A size of `0`
or `false` is not allowed, and the server does not start.

Only the person who runs the server can change this setting. A program cannot change it, so a
program cannot make its log fill the disk.

The example prints the setting and shows that a program cannot change it.
