Writes a list of dates with their time of day, in the way a language and region write them.

`Icu::formatDateTimes` takes a list of shapes, a locale tag and a shape of options. Each shape has the
local `year`, `month`, `day`, `hour`, `minute`, `second` and `nanos`, the `offsetSeconds` from UTC, and
the `zone` id, such as `"Europe/Vienna"`. The options are `length`, `seconds` and `zone`. The result
has one string for each value, in the same order.

`Icu::formatDateTimes` throws a `LogicError` when the locale tag or a value is not valid.

**Good to know:** most programs do not call `Icu::formatDateTimes` directly. `DateFormat::dateTimes`
takes `Core\Time\DateTime` values, builds these shapes for you, and calls it.
