Returns the subject of a received message, as a string.

A mail program writes a subject with letters outside ASCII as an encoded word, such as
`=?UTF-8?B?...?=`. An encoded word is the way a header writes text that is not ASCII. `subject`
decodes it, so the result is the text that the sender typed. A long subject can continue on the next
line, which starts with a space. `subject` replaces each line break and the spaces around it with
one space. A message without a `Subject` header returns an empty string.

**Good to know:** the subject is tainted, because the sender wrote it. Tainted means that the value
comes from outside your program, so you must check or escape it before you use it. The subject can
contain HTML, so escape it before you put it on a page.

**The examples below** read an encoded subject, find an order number in a subject, and show a
subject on a web page.
