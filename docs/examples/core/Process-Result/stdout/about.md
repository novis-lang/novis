Returns everything a program wrote to its standard output. The standard output is where a program
prints its normal results. Error messages go to the standard error instead, and `stderr` returns
those.

The result is `bytes`, not `string`. A program can write any data, for example an image or a
compressed file, and that data is often not valid text. When you know the output is text, use
`as string` to convert it. That conversion throws an error if the output is not valid UTF-8.

The program has already ended when you have a `Core\Process\Result`, so the whole output is there.
`stdout` never throws an error, and it returns the same output every time you call it.

**The examples below** read the output as text, read output that is not text, and use a system
tool to format a date.
