A day of the week: Monday, Tuesday, Wednesday, Thursday, Friday, Saturday or Sunday.

`Core\Time\Weekday` is declared in the namespace `Core\Time`, because the classes `DateTime` and
`Date` both use it. The methods `weekday`, `next` and `previous` use it.

Monday comes first, the way calendars in most of the world print it and the way the ISO week counts
it. Saturday and Sunday are the last two. The method `weekday` returns one of these cases. The methods
that move to a named day, such as the next Friday or the previous Monday, take one.

**Good to know:** a move to a named day never stays on the same day. The next Friday of a Friday is
one week later, which is what a person means by "next Friday". If you want the day itself when it
already matches, check the weekday first and move only when it does not match.
