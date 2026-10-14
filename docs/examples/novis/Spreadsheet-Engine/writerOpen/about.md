Starts a new workbook and returns a `Writer` for it.

`Engine::writerOpen` takes the document properties of the workbook and its `target`. The properties
are the title, the subject, the author, the company, the keywords and the time the workbook was
created, in UTC. `target` is a name for the workbook, such as its file name. If the workbook is
never finished, the error that is logged names it. The other `writer` functions return the bytes of
the file, and the program sends them where they go.

The workbook has no sheet yet, so `Engine::writerSheet` is the next call. A `created` time that is
not a real date and time throws a `LogicError`.

**Good to know:** most programs do not call `Engine::writerOpen` directly. `Workbook::createInMemory`,
`Workbook::createForFile`, `Workbook::createForStream` and `Workbook::createForDownload` call it
once, and send the bytes for you.
