Makes a `Row`, one row that a `Reader` read, from its two values.

`new Row` takes `number` and `values`, in that order. Each value becomes a read-only property of the
same name. `number` is the row number in the file, and the first row is 1. `values` has the values of
the row. The keys are the column letters, such as `A`, or the header text when you read with
`header: true`. A key is tainted (it came from the file), so check it before you trust it.

**Good to know:** `Reader::rows` makes a `Row` for you, one for each row it reads. You write
`new Row` yourself only when you need one without a file, for example in a test of your own code.
