How `DateFormat::dateTimes` shows the time zone of a date and time.

`DateFormat::ZoneStyle` is declared in the class `Novis\Intl\DateFormat`.

You pass a `DateFormat::ZoneStyle` as `zone`:

- `None` shows no time zone. This is the default.
- `Offset` shows the difference from UTC, such as "GMT+2".
- `Location` shows the country or city of the zone. For the zone `Europe/Vienna`, this is
  "Austria Time".
- `Generic` shows the general name of the zone, such as "Central European Time". It does not say
  whether it is summer time, so the name is the same in summer and in winter.

The time itself is always the local time of the value's own zone. The zone style only adds a label.

**Good to know:** show a zone whenever readers may live in different time zones, for example for an
online meeting.
