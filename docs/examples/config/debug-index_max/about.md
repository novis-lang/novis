The largest size of the database that `nvs debug` writes. The default is `1G`, which is one gigabyte.

`nvs debug` reads the records that `[debug] store` saves into one database, `debug/index.sqlite` in
the data folder. When the records in this database use more than `index_max`, `nvs debug` deletes
the oldest records first. It deletes a request or a finished run together with all of its records.

A command such as `nvs run` can still be running. For a run that has not ended, `nvs debug`
deletes its records one at a time, oldest first.

The file does not get smaller when records are deleted. New records use the space that the deleted
records used. The file stays about `index_max` in size.

`[debug] index_age` sets how long a record is kept. A record is deleted when it is past either
limit. A program cannot change this setting.
