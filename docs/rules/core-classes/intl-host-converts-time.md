`DateFormat::dateTimes` converts each `Core\Time\DateTime` in Novis, with `Core\Time`, to the record
`local-date-time` of `wit/intl.wit` — year, month, day, hour, minute, second, nanoseconds, the UTC
offset in seconds and the IANA zone id — and the guest formats those fields. `Core\Time` owns the
time-zone database, and the binary carries one.

`DateFormat::dates` and `times` cross as `nvs:ext/types`' `date` and `time-of-day` records, which are
local fields already. A `DateTime` does not cross as the `date-time` record, because that record is an
instant and a zone name, and turning it into fields is the time-zone database's job. An `Instant` is
placed in its zone with `Core\Time` first; a member that took an `Instant` and a `Zone` would be a
second way to write `dateTimes`.

The guest names a zone from its id and offset: `DateFormat::ZoneStyle::Offset`, `Location` or
`Generic`. A zone's
specific daylight-time name is not offered, because it needs to know whether the offset is daylight
time. The calendar is the locale's, selected by a tag's `-u-ca-` keyword; the fields cross as ISO
fields and the guest converts them.

**Declined:**

- **A second time-zone database in the guest**, so that a `DateTime` crosses as its instant. It doubles
  data `Core\Time` already carries, and two databases can disagree about the same instant.
