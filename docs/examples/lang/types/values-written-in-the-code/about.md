You can write a value directly in your program: a number, a string, `true`, `false` or `null`.

A number can have underscores between its digits, and it can be written in base 16, base 2 or
base 8. A leading zero has no special meaning, so `017` is seventeen. A number followed by a unit,
such as `30s` or `1h30m`, is a duration (a length of time).

A string in single quotes keeps every character as you typed it. A string in double quotes inserts
the values of your variables and reads escapes such as `\n`. A heredoc is a string that covers
several lines.

An html template, written ``html`...` ``, is a piece of a web page. Its type is `Core\Html\Markup`.
Every value you insert into it is escaped, so a value cannot add a tag.

**The examples below** show the ways to write a number, the two kinds of quotes, a message built
from a template, and a page fragment with inserted values.
