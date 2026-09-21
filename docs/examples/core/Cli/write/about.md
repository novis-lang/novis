Writes text to one of the terminal's standard streams.

Every program has two output streams. Standard output carries the result. Standard error carries the
messages meant for the person running the program. `Core\Cli::write` writes to either one, so a
result you send to a file stays clean while the progress notes still reach the screen.

`Core\Cli::write` writes exactly the characters you give it. It adds no newline. Ask for one with
`{newline: true}`, and build a line from as many calls as you like.

Text that can control the terminal is made safe first. A control character in a `string` is replaced
by a visible character that does nothing, the same replacement `Core\Cli::escape` makes. A
`Core\Cli\Text` is written as it is, because its styling is there on purpose.

**Good to know:** standard input is not a stream a program writes to. `Core\Cli\Stream::In` throws an
error.

**The examples below** build a line from pieces, send progress to standard error, and print a report
whose columns line up.
