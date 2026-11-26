Returns the status code that the mail server gave for one recipient of a delivery report, such as `5.1.1`.

The code is three numbers joined by dots. The first number is the class: `2` means the mail was delivered, `4`
means a temporary problem, and `5` means a permanent failure. The second and third numbers say what
went wrong. For example, `5.1.1` means the address does not exist, and `4.2.2` means the mailbox is
full for now.

The code comes from the `Status` line of the report. The result is the first word of that line, so
a comment after the code is not part of it. When the report has no such code, the result is an
empty string.

**Good to know:** the server that sent the report wrote this text, so the result is a tainted
string. Tainted means that the value comes from outside your program, so you must check or escape
it before you use it. Compare it with the codes you know. Escape it before you show it on a page.
