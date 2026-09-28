Returns the part of a string that comes before a separator, without the separator itself.

`Core\Str::before` looks for the first place the separator appears and returns everything to its
left. If the separator is not in the string, the result is `null`. If the separator is at the very
start, the result is an empty string. So you can always tell "not found" apart from "found, with
nothing before it". With the option `{last: true}`, it cuts at the last place the separator appears.

The search is case-sensitive. `Core\Str::after` does the same for the part on the right. Together
they replace PHP's `strstr` and `strrchr`.

**The examples below** read the key out of a line, remove a file's extension with `{last: true}`,
and take the user name out of an email address.
