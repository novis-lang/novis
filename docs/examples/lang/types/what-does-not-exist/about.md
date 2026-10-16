A short list of PHP spellings Novis does not have, and the one thing to write in place of each.

Casts such as `(int)$x` are gone, and so are `intval`, `gettype`, `is_int` and `settype`. Conversion
is written `$x as int`, and it is checked: a value that will not fit throws instead of quietly
becoming something else, and `$x as ?int` answers `null` where you would rather ask than catch.
There are no free functions at all, so a test for a type is `is` for a class and `($x as ?int) !=
null` for a number. A string is read with `Core\Str::at` and `Core\Str::slice` rather than `$s[0]`,
ordered with `Core\Str::compare` rather than `<`, and never takes part in arithmetic. Destructuring
is written with brackets, and every leaf names its type.

**Good to know:** every one of these is a message from the compiler that names the replacement, so a
program brought over from PHP shows you the whole list before it runs once.
