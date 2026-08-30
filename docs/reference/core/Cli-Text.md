---
summary: the value a captured terminal write comes back as — bytes that have already been through the output sink
keywords: Core\Out::capture, ob_start, ob_get_clean, terminal output, ANSI, escape sequences, captured output, carrier
---

`Core\Cli\Text` is what `Core\Out::capture` answers when the program is not serving an HTTP request: the
bytes the captured code wrote, already past the terminal sink, carried as a value rather than as a `string`
so that writing them out again does not escape them a second time. **It has no members and no
constructor.** A `Text` is produced by `Core\Out::capture`, written by `echo`, joined with `.` like any other
value, and turned into a `string` with `as string`; a parameter or binding declared `string` refuses it
until it is converted. Styling, colour and the rest of a terminal surface are not on this class.

```nvs
<?nvs
var $captured = Core\Out::capture(fn (): void => {
    echo "inner";
});

echo "[", $captured, "]\n";
echo "joined: " . $captured . "!\n";

string $text = $captured as string;
echo "length=", Core\Str::length($text), "\n";
```
```output
[inner]
joined: inner!
length=5
```
