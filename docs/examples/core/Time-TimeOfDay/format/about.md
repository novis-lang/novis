Returns the time of day as text, in a pattern you choose.

The pattern uses letters for the parts of the time. `HH` is the hour from 00 to 23, `h` is the
hour from 1 to 12, `mm` is the minute and `ss` is the second. `a` is AM or PM, and `SSS` shows
milliseconds. Any other text in the pattern goes into the result as it is. The letters are the
same ones `DateTime::format` uses.

A `TimeOfDay` has no date and no time zone. So a pattern with a letter for the year, the day or
the zone throws a `LogicError`. The pattern must come from your program: a `tainted` pattern (text
that came from a request or another outside source) does not compile.
