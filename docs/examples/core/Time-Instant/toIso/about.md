Returns this instant as text in the RFC 3339 format, always in UTC.

The text looks like `2024-03-01T12:00:00Z`. The `Z` at the end means UTC. Most JSON and HTTP
APIs expect dates in this format, and `Core\Time::fromIso` reads it back into the same instant.
The seconds have a fraction only when the fraction is not zero. For example, half a second
past noon is written `12:00:00.5Z`.

An instant has no time zone, so the text is the same wherever the program runs. To show the
time on a local clock, use `in` with a `Zone` first.
