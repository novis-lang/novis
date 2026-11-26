Returns the address that the mail server tried to deliver to, such as `ada@example.org`.

The address comes from the `Final-Recipient` line of the delivery report. The type in front of
it, such as `rfc822;`, and any angle brackets around it are removed. So you can compare the
result with the addresses you sent mail to.

**Good to know:** the result is a `tainted` string, because the server that sent the report
wrote it. Use it to find an address you already have, and change only your own copy. A recipient
whose address is empty or has a control character is not in the report's list.

**The examples below** print the address of each recipient, show the brackets and the type being
removed, and find the customers whose address failed.
