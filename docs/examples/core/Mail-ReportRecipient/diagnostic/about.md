Returns the message that the receiving mail server gave for one recipient of a delivery report, such as `550 5.1.1 User unknown`.

The text comes from the `Diagnostic-Code` field of the report, without the `smtp;` type in front
of it. Each server writes its own words, so the text is meant for a person to read. When the report
has no `Diagnostic-Code`, the result is `null`.

**Good to know:** a program should decide what to do with `status()` and `isPermanent()`, and keep
this text for a log or a support page. The server that sent the report wrote it, so the result is a
tainted string. Escape it before you show it on a page.
