Finishes the last sheet and the workbook, and returns the end of the file.

`Engine::writerFinish` takes a `Writer`. It writes the rest of the last sheet, the parts that wait
for the end of the workbook, and the index of the zip file. The result has `data`, the last bytes
of the file, and `rows`, the name of each sheet with its number of rows. `size` is the size of the
whole file in bytes, so a program can check it against what it sent.

A workbook with no sheet, and a workbook where every sheet is hidden, throw a `LogicError`. A
second call throws a `LogicError` too. So do `Engine::writerRows` and `Engine::writerDefine` after
the workbook is finished.

**Good to know:** most programs do not call `Engine::writerFinish` directly. `Workbook::finish`
calls it once and returns a `Written` with the row counts and the size.
