How long `nvs debug` keeps the record of a request. The default is `7d`, which is seven days.

`nvs debug` reads the records that `[debug] store` saves into one database, `debug/index.sqlite`.
This file is in the data folder. The data folder is `.nvsdata` next to the `nvs` program, unless
`--data` names another folder.

Each record belongs to one request or one run. After `nvs debug` reads new files, it deletes
every request and run that started more than `index_age` ago, with all of their records.

A command such as `nvs run` can run for longer than `index_age`. For a run that has not ended,
`nvs debug` deletes only the records that are older than `index_age`.

When `nvs serve` starts `nvs debug`, `nvs debug` uses the same configuration files as the server.
When you start `nvs debug` yourself, it reads its configuration the same way as every other `nvs`
command. A program cannot change this setting.

A record can contain data that a user sent, such as the body of a form. Keep `index_age` only as
long as you need the records.
