Reads a CSV file one record at a time, so the whole file never has to be in memory.

`Core\Csv::rows` takes a file that `Core\IO::open` has opened for reading. You read the result with
a `foreach` loop, and every step gives you one record as an array of text fields. Only one record is
kept at a time, so a file larger than the memory your program has still reads.

Reading starts at the position the file is already at, and the result can be read only once. Set
`header` to `true` and the first record becomes the column names. Every record after it is keyed by
those names, and the header is not one of the records you read.

The `separator`, `quote` and `escape` options read a file written with other characters, and are
the three `Core\Csv::parse` reads.

**Good to know:** the file stays yours. `Core\Csv::rows` opens nothing and closes nothing, so close
the handle yourself when you are finished with it.
