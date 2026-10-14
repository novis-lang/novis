Writes the rest of a filled template and returns the end of the file.

`Engine::templateFinish` takes a `Filling` and its source, the same template that
`Engine::templateOpen` read. It writes the rows after the last list and every other part of the
file. The result has `data`, the last bytes of the file, and `rows`, the name of each sheet with its
number of rows. `size` is the size of the whole file in bytes.

A template that is not filled throws a `LogicError`, and so does a second call. A placeholder with
no value throws a `LogicError` here too. A source that is not `bytes` or a file throws a
`LogicError`, and another workbook as the source can throw a `ParseError`.

**Good to know:** most programs do not call `Engine::templateFinish` directly. When the `Export`
from `Template::fill` is written, it calls this once, after the last items.
