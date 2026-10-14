Opens a template and returns a `Filling` for it.

`Engine::templateOpen` takes the source and two bounds. The source is the bytes of an xlsx or xlsm
file, or a `Core\IO\File` the program opened. A template is a workbook with placeholders such as
`{{customer.name}}` in its cells. The other `template` functions fill the placeholders and write
the file.

The bounds are the same as for `Engine::readerOpen`: `maxBytes` for a part read whole, and
`maxRatio` for how far a part decompresses. A part past a bound, a CSV file and a file that is not
a workbook throw a `ParseError`. A source that is not `bytes` or a file throws a `LogicError`.

**Good to know:** most programs do not call `Engine::templateOpen` directly. `Template::open` calls
it once.
