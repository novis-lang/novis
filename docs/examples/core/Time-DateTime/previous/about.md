Returns a new date and time on the last earlier day that falls on a given weekday.

You give a case of `Core\Weekday`, such as `Core\Weekday::Friday`. The result is the nearest
earlier day with that weekday. The time of day and the time zone stay the same, and the value you
call `previous` on does not change.

`previous` always moves back, by one to seven days. When the value is already on that weekday, the
result is seven days earlier.

**Good to know:** on a Monday, `previous` with `Core\Weekday::Monday` gives the Monday of the week
before. To find the Monday that a week started on, check the weekday first and keep a Monday as it
is.
