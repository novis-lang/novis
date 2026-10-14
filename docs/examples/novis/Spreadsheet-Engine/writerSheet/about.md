Starts a new sheet in a workbook and returns the bytes of the file that are ready to send.

`Engine::writerSheet` takes a `Writer`, the options of the sheet and the rows in `first`. It
finishes the sheet before it, if there is one. The options are the name, the tab position, whether
the sheet is hidden, the columns, the frozen rows and columns, the autofilter, the tab color, the
zoom, and how the header row looks.

A sheet name has 1 to 31 characters and no `: \ / ? * [ ]`. It does not start or end with `'`,
and no other sheet has the same name in any case. A zoom is from 10 to 400, and a column width is
from 0 to 255. A sheet has at most 16,384 columns. Any other value throws a `LogicError`, and so
does a call after `Engine::writerFinish`.

**Good to know:** most programs do not call `Engine::writerSheet` directly. `Workbook::sheet` calls
it once for each sheet, with no rows in `first`.
