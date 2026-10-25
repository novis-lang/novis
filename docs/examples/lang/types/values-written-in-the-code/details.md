A float can also be written `1E-2` or `1_000.5`. The keywords `true`, `false` and `null` are values
too.

A single-quoted string has two escapes: `\\` is one backslash and `\'` is a quote. Every other
backslash is printed as it is, so `'a\nb'` prints `a\nb`.

A double-quoted string has the escapes `\\`, `\"`, `\$`, `\n`, `\t`, `\r`, `\v` and `\f`. It also
has an octal escape from `\0` to `\777`, `\xHH` and `\u{HHHH}`. An octal or `\x` escape writes one
byte. The bytes of the string must form valid UTF-8, so `"\xff"` is the error `E0431`. An escape the
language does not know keeps its backslash, so `"\q"` prints `\q`.

A string cannot contain the `ESC` character. This is the error `E0011`. It is the same for the raw
byte and for `\e`, `\x1b`, `\033` and `\u{1b}`. To style text in a terminal, use `Core\Cli\Text`.

In a double-quoted string, `$name[key]` and `$name[0]` read one array element. A second `[...]`
after it is text, so `"$s[0][1]"` prints the element and then `[1]`. For more, use braces. The
expression in braces must start with a variable, and then it can be any expression, such as
`{$a + $b}`. A `{` with no `$` after it is text. `"${name}"` prints `${name}`.

A value in a string is converted the way `echo` converts it. A number, a `bool`, a `string` and
`null` work. `bytes`, an array and an enum case are the error `E0707`. An object of a class that
does not implement `Stringable` is the error `E0412`.

In a heredoc, every line must start with the same spaces as the closing marker. Those spaces are
removed. A line with less indentation is the error `E0433`.

A duration has the units `ns`, `us`, `ms`, `s`, `m`, `h`, `d` and `w`. A duration is an object, so
`==` checks whether two variables are the same object. `1h == 1h` is `false`. To compare two
durations, use `compareTo`, which returns `0` when they are equal.

An anonymous object at the start of a statement must be in parentheses: `({ x: 1 });`. Without them
it is the error `E0117`. The same is true for the body of an arrow function.

An html template keeps every byte between the backticks, also the spaces and line breaks. Write a
backtick in it as `` \` ``. Write `\{` for a `{` that opens no hole. A variable after it is still
filled in, so `\{$n}` prints `{`, the value and `}`.

A brace that is not followed by `$` is text. `{Page::TITLE}` prints as it is written, and the
compiler gives the warning `W1012`. Write `<?= Page::TITLE ?>` for a constant or a method call.

`<?= ... ?>` in an html template takes any expression. Spaces around it are allowed. A `}` inside it
is an ordinary brace. The first `?>` that is not inside a string or a template closes it. A `<?nvs`
block inside a template is the error `E0010`. A template is one expression. To repeat markup in a
loop, build the fragments in code and join them.

A `string` in a hole is escaped, also when it is tainted. A `Core\Html\Markup` in a hole is written
as it is. A secret value in a hole is the error `E0790`. A value with no string form is the error
`E0707`.

`+` joins two `Core\Html\Markup` values. `Core\Html::join($parts, $separator)` joins a list of
them. `.` with a `Core\Html\Markup` is the error `E0710`. The text it would make is escaped a second
time when it is printed.

`html"..."` with double quotes does not compile. The error `E0319` says to write it with backticks.

`"<p>x</p>" as Core\Html\Markup` converts a string that is written directly in the code. Any other
string is the error `E0417`. To turn a computed string into markup, use `Core\Html::escape` or
`Core\Html::sanitize`.
