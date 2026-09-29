Makes a time zone that is always the same distance from UTC, such as `+05:30` or `-03:00`.

You give the distance as a `Duration`. It is positive for a zone east of UTC and negative for a zone
west of UTC. The zone's name is the offset itself, for example `+05:30`. The offset never changes,
so a fixed zone has no summer time. A named zone such as `Europe/Berlin` changes its offset twice a
year, and a fixed zone does not.

Use a fixed zone when you only know the offset and not the name of the place. A timestamp such as
`2024-03-05T10:00:00+05:30` is one example. The offset must be a whole number of seconds, and at
most 25 hours, 59 minutes and 59 seconds away from UTC. Any other offset throws a `RuntimeError`.
