---
summary: the lines of a text file, which `Core\IO::lines` returns. You read them with a `foreach` loop
keywords: read lines, read a file line by line, lines of a file, text file, line endings, line break, iterate lines
---

`Core\IO\Lines` contains the lines of a text file. `Core\IO::lines` returns it. The class has no
methods. You read it with a `foreach` loop.

Each item is a `string` with one line, without its line break. A line ends at `\n`, `\r\n` or a single
`\r`. An empty line in the middle of the file is an empty string. A line break at the end of the file
does not add an empty last line.

`Core\IO::lines` reads the whole file and closes it before it returns. A change to the file after
that does not change the result. You can loop over the result as often as you like. Each loop starts
at the first line again.

`Core\IO::lines` throws an `IOError` when the file does not exist or cannot be read. It throws a
`RuntimeError` when the file is not valid UTF-8 text, or when the program may not read the path. The
loop itself does not throw an error.

```txt file=todo.txt
buy milk
call the bank
water the plants
```
```nvs
<?nvs
var $lines = Core\IO::lines('todo.txt');

uint $number = 0;
foreach ($lines as string $line) {
    $number = $number + 1;
    echo $number, ': ', $line, "\n";
}

// A second loop starts at the first line again.
foreach ($lines as string $line) {
    echo Core\Str::length($line), ' ';
}
echo "\n";
```
```output
1: buy milk
2: call the bank
3: water the plants
8 13 16
```
