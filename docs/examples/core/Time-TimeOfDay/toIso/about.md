Returns the time as ISO 8601 text, such as `08:30:00`.

The text has the hour, the minute and the second, each with two digits. It has a fraction of a
second only when the fraction is not zero, such as `12:00:05.25`. Zeros at the end of the fraction
are left out.

`Core\Time\TimeOfDay::fromIso` reads this text back, and the result is the same time. Use `toIso` to
save a time in a file or a database, or to send it to another program. For text a person reads,
use `format`, such as `format('h:mm a')`.

`Core\Json::encode`, `Core\Router::url` and `Core\Uri::buildQuery` write a `TimeOfDay` as this
text. You do not need to call `toIso` for them.
