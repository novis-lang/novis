Splits a string into parts at a separator.

`Core\Str::split` takes a string and a separator, and returns an array of the parts between the
separators. The separator itself is not in any part. Two separators next to each other give an
empty part, and the empty string gives one empty part. The separator must not be empty. An empty
separator throws a `RuntimeError`.

The `limit` option controls how many parts you get. A positive limit gives at most that many parts,
and the last part contains the rest of the string. A negative limit deletes that many parts from
the end. A limit of `0` gives the whole string as one part.

The examples split a list of tags, remove the extension from a file name with a negative limit,
and read the fields of one line of a log file with a positive limit.
