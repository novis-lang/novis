Returns the whole spreadsheet as `bytes`, kept in memory.

Use it when the file goes somewhere other than a download or a path: into a database, an email
attachment or a JSON response. The format is Excel (xlsx) unless the export was made with the option
`format: "csv"`. You can read the bytes back with `Reader::open`.

**Good to know:** the whole file is in memory until your program is done with it. For a large
export, `download` and `save` send each part of the file as soon as it is ready. An export from
`fromRows` with an array writes the same file each time you call it. A generator can be read only
once, so a second call on that export throws a `LogicError`.
