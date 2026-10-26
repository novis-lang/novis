`nvs debug` reads the debug records of your requests into one database.

With `[debug] store = true` in `nvs.toml`, Novis writes a record of each request to files in
`debug/spool` in the data folder. `nvs debug` reads each file when it is complete. It saves the
records in `debug/index.sqlite` and deletes the file. Then it waits for new files. Files that were
written while `nvs debug` did not run are read when it starts.

In development mode, `nvs serve` starts `nvs debug` when `store` is on. It stops `nvs debug` when the
server stops. A production server never starts it. On a production host, you start `nvs debug`
yourself.

`--once` reads the files that are ready, deletes old records, and exits. `--tail <file>` also reads a
log file that uses the `json` log format, and each line that is added to it later. `nvs debug` never
changes that file.

Only one `nvs debug` runs for a data folder. A second one prints an error and exits with status `1`.
