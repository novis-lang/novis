Checks what a person types into the cells of `$range` in Excel, and returns the sheet.

`$rule` has one of these: `list` is the values a person can choose from. `whole` is a whole number,
`decimal` any number, `date` a date and `length` the length of a text. Each of these four takes
`min`, `max` or both. `formula` is an Excel formula that must be true. `input` and `inputTitle` are a
message Excel shows when the cell is selected. `error` and `errorTitle` are the message for a wrong
value. `allowEmpty` is `true` unless you set it to `false`.

**Good to know:** Excel checks only what a person types. A program that reads the file later must
still check every value. A rule with none or two of the kinds, or a number rule without `min` and
`max`, throws a `LogicError`. A CSV file has no validation, so `validate` does nothing there.

**The examples below** offer a list of choices, allow a whole number between two limits, and show
the error for a rule with two kinds.
