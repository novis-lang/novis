Says what a result column was declared as in the database.

Every read hands back a description of its columns beside the rows, and this is the type half of that
description. It describes the column rather than the values: a column declared as JSON and one declared
as text both read back as text, and this is what tells them apart. There is one case for each kind of
column Novis has a type of its own for — whole numbers, exact decimals, text, raw bytes, booleans, the
four date and time shapes, UUIDs and JSON — and `Other` for everything else, including every array
column. A program that renders, exports or paginates rows reads this to decide how to treat each column
instead of guessing from whichever values it happened to receive.

**Good to know:** the cases are a set and not a scale, so asking whether one is greater than another
means nothing; the only useful question is which case a column is. A backend with no JSON type of its
own reports its JSON columns as text.
