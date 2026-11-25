Reads a time of day written as text, such as `09:30` or `09:30:15.5`, and returns a `TimeOfDay`.
The text must follow ISO 8601, the international standard way to write a date and time.

The text is `HH:MM`, `HH:MM:SS` or `HH:MM:SS` with a fraction of a second. The fraction has one to
nine digits after the `.`, so the smallest step is one nanosecond. The hour is `00` to `23`, and the
minute and the second are `00` to `59`. So `24:00` and a second of `60` are not allowed.

A text that does not match throws a `ParseError`, and the message says which part is wrong. A text
written directly in the code is checked when the program compiles, so `TimeOfDay::fromIso('25:00')`
does not compile. A text in a variable is read when the program runs.

The same rules read a `TimeOfDay` in a route path, a `#[Core\Query]` value, a command argument and
JSON that `Core\Json::decodeAs` reads.

**The examples below** show the three forms of the text, texts that are not valid times, and opening
hours read from a settings file.
