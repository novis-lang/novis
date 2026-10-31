What a data validation checks when a person types into a cell in Excel.

You add a validation with `Sheet::validate`. Its rule has exactly one of these options, and the
option gives the `Sheet::RuleKind` case:

- `list` is `List`. The value must be one of the strings in the list.
- `whole` is `Whole`, a whole number.
- `decimal` is `Decimal`, any number.
- `date` is `Date`, a date.
- `length` is `Length`. It checks the number of characters of the text.
- `formula` is `Formula`. The value is allowed when the formula is true.

`whole`, `decimal`, `date` and `length` take `min`, `max` or both. A rule with no option or with two
options throws a `LogicError`.

**Good to know:** Excel checks only what a person types. A file that your program reads back can
still have any value, so check the values again in your program.
