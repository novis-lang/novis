The largest size of the database that `nvs debug` writes. The default is `1G`.

`nvs debug` reads the records that `[debug] store` saves into one database, `debug/index.sqlite` in
the data folder. When the data in the database is larger than `index_max`, `nvs debug` deletes the
oldest records first. A request or a finished run is deleted with all of its records. The records of
a command that is still running are deleted one by one. New records then use the free space, so the
file stays close to this size.

`[debug] index_age` sets how long a record is kept. A record is deleted when it is past either
limit. A program cannot change this setting.
