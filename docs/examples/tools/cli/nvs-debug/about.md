`nvs debug` reads the debug records of your requests and runs into one database.

With `[debug] store = true` in `nvs.toml`, Novis writes a record of each request, `nvs run` and
`nvs test`. The records go to files in `debug/spool` in the data folder. Novis finishes a file
before `nvs debug` reads it. `nvs debug` saves its records in `debug/index.sqlite` and deletes the
file. Then it waits for new files. When `nvs debug` starts, it reads the files that were written
while it was not running.

In development mode, `nvs serve` starts `nvs debug` when `store` is on, and stops it with the server.
On a production host, you start `nvs debug` yourself.

`--once` reads the files that are ready, deletes old records, and exits. `--tail <file>` reads a
log file too. `nvs debug` reads the lines that are in the file, and each line that is added later.
Set `[log] format = "json"` for that log. `nvs debug` never changes the file.

Only one `nvs debug` runs for a data folder. A second one prints an error and exits with status `1`.
