`Core\Taint::assertTrusted()` returns a `tainted string` as a plain `string`, after your own check.

A value that comes from outside your program, such as a visitor's request, has the type `tainted
string`. Functions that run text as code, such as a regular expression or a database query, do not
accept it. Usually you use a function made for that one place, such as `Core\Html::escape`. When
none fits and your program has checked the value itself, you call `assertTrusted()`.

Every call needs a second argument: a short reason that says what was checked. The program never
reads it. The text does not change at all. Nothing is escaped or removed.

**In plain words:** a signed note from you. It says "I checked this", and anyone who reads the code
can find it by name.

**The examples below** show a value compared with a list of allowed values, the text staying exactly
the same, and a search pattern from the settings used as a regular expression.
