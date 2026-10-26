How long `nvs debug` keeps the record of a request. The default is `7d`, which is seven days.

`nvs debug` reads the records that `[debug] store` saves into one database, `debug/index.sqlite` in
the data folder. The age of a record starts at the start of its request. Each time `nvs debug` has
read new files, it deletes the requests that are older than `index_age`, with all of their records.

`nvs debug` reads this setting from the configuration of the server that started it. When you start
`nvs debug` yourself, it reads its configuration the same way as every other `nvs` command. A
program cannot change this setting.

**Good to know:** a record can contain data that a user sent, such as the body of a form. Keep
`index_age` only as long as you need the records.
