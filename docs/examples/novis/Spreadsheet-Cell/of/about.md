Gives one value in a row its own style, such as a red total or a highlighted name.

Put the result of `Cell::of` in a row in place of the value. The value is written the same way as
without a style. For each style field, the cell uses its own style first, then the style of its row,
then the style of its column. So a cell can change one field, such as `fill`, and keep the other
fields of its row.

**Good to know:** a CSV file has no styles. There, the cell is written as its value alone.
