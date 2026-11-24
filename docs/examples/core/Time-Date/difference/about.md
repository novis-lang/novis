Counts the whole days, weeks, months, quarters or years from one date to another. You choose the
unit with a case of `Core\Unit`: `Unit::Day`, `Unit::Week`, `Unit::Month`, `Unit::Quarter` or
`Unit::Year`.

`$start->difference($end, Unit::Day)` counts from `$start` to `$end`. The result is positive when
`$end` is later and negative when `$end` is earlier. Only whole units count, so the result is `0`
when the two dates are less than one unit apart.

A month counts when the same day of the month is reached. From 31 January to 29 February is `0`
months, because February has no day 31. From 31 January to 1 March is `1` month. `difference`
counts the same way as `Core\Time\DateTime::difference`, and it never throws an error.

Use `Unit::Year` for the age of a person, and `Unit::Month` for the length of a contract.

**Good to know:** a date has no time of day, so units smaller than `Unit::Day` do not compile.
