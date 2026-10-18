A program ends when its last line has run. Its exit status is then `0`, which tells the shell that
all went well.

`exit` ends the program early. `exit;` ends it with status `0`. `exit(3);` ends it with status `3`,
and the shell can read that number. A status is a number from 0 to 255. A program that checks or
imports something can use it to report how many problems it found. `exit("Nothing to do.");`
prints the text and ends with status `0`.

**Good to know:** `exit` is not an error. A `catch` does not stop it, and a `finally` block does not
run. `die` does not exist in Novis. If an error is thrown and nothing catches it, the program ends
with status `1` and prints where the error came from.
The last example shows that a line after `exit` does not run.
