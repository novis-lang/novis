Writes a list of times of day in the way a language and region write them.

`Icu::formatTimes` takes a list of `Core\Time\TimeOfDay` values, a locale tag and a shape of options.
The only option is `seconds`. It returns one string for each time, in the same order. In American
English, half past nine in the evening gives "9:30 PM".

`Icu::formatTimes` throws a `LogicError` when the locale tag is not valid.

**Good to know:** most programs do not call `Icu::formatTimes` directly. `DateFormat::times` is the
same function.
