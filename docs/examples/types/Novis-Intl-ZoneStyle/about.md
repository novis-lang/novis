How `DateFormat::dateTimes` shows the time zone of a date and time.

You pass a `ZoneStyle` as `zone`:

- `None` shows no time zone. This is the default.
- `Offset` shows the difference from UTC, such as "GMT+2".
- `Location` names a place, such as "Austria Time".
- `Generic` names the zone, such as "Central European Time".

The time itself is always the local time of the value's own zone. The zone style only adds a label.

**Good to know:** show a zone whenever readers may live in different time zones, for example for an
online meeting.
