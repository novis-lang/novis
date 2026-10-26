The largest size of the database that `nvs debug` writes. The default is `1G`.

`nvs debug` reads the records that `[debug] store` saves into one database, `debug/index.sqlite` in
the data folder. When the data in the database is larger than `index_max`, `nvs debug` deletes the
oldest requests first, with all of their records. New records then use the free space, so the file
stays close to this size.

`[debug] index_age` sets how long a record is kept. A request is deleted when it is past either
limit. A program cannot change this setting.
