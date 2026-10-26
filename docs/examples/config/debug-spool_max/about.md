The most disk space that the debug records of one process may use. The default is `256M`.

With `[debug] store = true`, Novis writes a record of each request to files in `debug/spool` in the
data folder. `nvs debug` reads these files and deletes them. When `nvs debug` does not run, the files
stay on the disk. This limit stops them from filling the disk.

A server divides the space equally between its worker threads, and each thread writes its own files.
When a thread needs more space than its part, it deletes its own oldest file. It then writes a line
with the number of records it deleted. `nvs debug` saves that number in its database.

A program cannot change this setting. You can change it in `nvs.toml` while the server runs.
