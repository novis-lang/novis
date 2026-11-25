Returns the date as text in the ISO 8601 form `YYYY-MM-DD`, such as `2026-05-20`.

The month and the day always have two digits. The year always has four digits, so the year 5 is
written `0005`. Year 0 is `0000`. A year before year 0 starts with `-`, such as `-0044-03-15`.

`Core\Time\Date::fromIso` reads this text back, and the result is the same date. That makes
`toIso` the right choice when you save a date to a file or a database. ISO text also sorts in date
order as a plain string.

`Core\Json::encode`, `Core\Router::url` and `Core\Uri::buildQuery` write a `Date` as this text. You
do not need to call `toIso` for them.

**Good to know:** to write a date in another form, such as `20 May 2026`, use `format`.
