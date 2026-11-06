Reads CSV data one record at a time, so the whole file never has to be in memory.

`Core\Csv::rows` reads from an open file, an uploaded file, `Core\Request::bodyStream()`, `bytes`,
or a list or loop that gives `bytes` pieces. Pass the file or the upload directly. Do not read it
into a variable first. You read the result with a `foreach` loop, and each step gives you one record
as an array of text fields. Only one record is kept at a time.

A file is read from its first byte. You can loop over the result only once. Set `header` to `true`
and the first record gives the column names. Every record after it uses those names as keys.

The `separator`, `quote` and `escape` options are the same as for `Core\Csv::parse`. Set `charset`
for a file that is not UTF-8, such as `Core\Charset::Windows1252`. A byte order mark at the start
sets the charset for you.

**Good to know:** `Core\Csv::rows` does not open or close a file. Close it yourself when the loop is
done.
