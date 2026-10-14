Returns the number of a cell style in a workbook.

`Engine::writerStyle` takes a `Writer` and the fields of one style: the font, its size, bold,
italic, colors, borders, alignment, rotation, indent, a number format and protection. A field that
is `null` keeps the default. Two calls with equal fields return the same number. A cell, a row or a
column in `Engine::writerRows` and `Engine::writerSheet` uses this number as its `style`. A style is
written to the file only when a cell uses it.

A color is written as `#RRGGBB`. A font size is from 1 to 409, a font name has 1 to 31 characters,
a rotation is from -90 to 90, an indent is at most 250, and a number format has 1 to 255
characters. Any other value throws a `LogicError`.

**Good to know:** most programs do not call `Engine::writerStyle` directly. `Sheet::add` and
`Sheet::addAll` call it the first time they write a `Style` made with `Style::of`.
