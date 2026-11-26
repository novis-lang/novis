How long `nvs debug` keeps the record of a request. The default is `7d`, which is seven days.

`nvs debug` reads the records that `[debug] store` saves into one database, `debug/index.sqlite`.
This file is in the data folder, `.nvsdata` next to the `nvs` program.

Each record belongs to one request or one run. A run is a scheduled job, a queue job, or a command
such as `nvs run`. After `nvs debug` reads new records, it deletes each request that started more
than `index_age` ago. All the records of that request are deleted with it.

Each record of a run is deleted when it is older than `index_age`. A run can last longer than
`index_age`. Then the run loses its first records and keeps its newest ones. The run itself is
deleted when it ended more than `index_age` ago. The page of the run in the debug viewer says which
records were deleted.

When `nvs serve` starts `nvs debug`, `nvs debug` uses the same configuration files as the server.
A program cannot change this setting.

A record can contain data that a user sent, such as the body of a form. Set `index_age` no longer
than you need the records.
