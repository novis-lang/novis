Returns the time zone of a date and time, as a `Core\Time\Zone`.

Every date and time is in one time zone. It is the zone you gave when you created the value, or the
zone you converted a moment to. `zone` returns it, so you can use it again for another value.

A common use is to show other times in the same zone. For a booking in Sydney, take the zone of the
booking and convert each reminder time to it.

**Good to know:** a zone with a place name, such as `Europe/Berlin`, follows the clock changes of
that place. A zone made from a fixed offset, such as `+05:30`, has the same offset all year.
