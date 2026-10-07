`DateFormat::dateTimes` converts each `Core\Time\DateTime` in Novis, with `Core\Time`, to the record
`local-date-time` of `wit/intl.wit` — year, month, day, hour, minute, second, nanoseconds, the UTC
offset in seconds and the IANA zone id — and the guest formats those fields. `Core\Time` owns the
time-zone database, and the binary carries one.

`DateFormat::dates` and `times` cross as `nvs:ext/types`' `date` and `time-of-day` records, which are
local fields already. An `Instant` is placed in its zone with `Core\Time` first; there is no member that
takes an `Instant` and a `Zone`.

The guest names a zone from its id and offset: `ZoneStyle::Offset`, `Location` or `Generic`. A zone's
specific daylight-time name is not offered. The calendar is the locale's, selected by a tag's `-u-ca-`
keyword; the fields cross as ISO fields and the guest converts them.
