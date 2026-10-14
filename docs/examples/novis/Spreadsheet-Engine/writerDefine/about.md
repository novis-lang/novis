Gives a range of cells a name that formulas can use.

`Engine::writerDefine` takes a `Writer`, a name and a reference, such as `Sheet1!$A$1:$B$10`. A
formula in the workbook can then use the name in place of the reference. The name is written to
the workbook when it is finished, and the reference is written as escaped text.

A name starts with a letter or `_`, and has only letters, digits, `_` and `.`. A name that is a
cell address such as `B2`, an empty reference, a name used twice in any case, and the name of a
table throw a `LogicError`. So does a call after `Engine::writerFinish`.

**Good to know:** most programs do not call `Engine::writerDefine` directly. `Workbook::define`
calls it once for each name.
