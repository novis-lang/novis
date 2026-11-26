Returns the address that the mail server tried to deliver to, such as `ada@example.org`.

The address comes from the `Final-Recipient` line of the delivery report (the address the server
tried). A delivery report is the standard part of a bounce that lists each recipient and what
happened. The type in front of the address, such as `rfc822;`, and any angle brackets around it are
removed. So you can compare the result with the addresses you sent mail to.

**Good to know:** the result is a `tainted` string, because the server that sent the report
wrote it. Tainted means that the value comes from outside your program, so you must check or escape
it before you use it. Use the address to find the row in your own data, and change that row. A
recipient whose address is empty or has a control character is not in the report's list.

**The examples below** print the address of each recipient, show the brackets and the type being
removed, and find the customers whose address failed.
