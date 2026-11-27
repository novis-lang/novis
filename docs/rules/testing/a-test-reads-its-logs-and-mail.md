`Core\Test::logs(): array<Core\Test\LogRecord>` returns every log record written since the test
started, by the test, its requests and its runs, in the order written. `Core\Test\LogRecord` has
`level(): string`, `message(): string` and `fields(): array<mixed>`. Records still reach their `[log]
target`, so a test's report and a run's log file read the same. What it spends: the records of one
test, freed with it.

**Under `nvs test`, `Core\Mail` sends nothing.** `Core\Mail::send` keeps the message and opens no
connection, and `Core\Test::sentMail(): array<Core\Test\SentMail>` returns the messages sent since
the test started, by the test, its requests and its runs, in the order sent. `Core\Test\SentMail`'s
members are the fields `Core\Mail::send` takes, read back, and what it wrote: `messageId()`,
`fromName()`, `inReplyTo()`, `references()`, `unsubscribe()`, `isAutoSubmitted()`, `bounceTag()`, and
`attachments()`, each a `Core\Mail\Attachment` with `name()`, `type()`, `bytes()` and `isInline()`.
`html()` is the HTML as sent, its CSS inlined, and `text()` the plain text as sent, built from the
HTML where the program gave none. A `[mail.<name>] capture = true` block keeps its mail the same way
outside a test (`rule:core-classes/mail-transport`). It is the reason an outbound HTTP call is
answered from a table in a test (`rule:testing/an-outbound-call-is-answered-from-a-table`): a test
never reaches a real person. What it spends: the messages of one test, freed with it.

A test that reads mail opens an `[imap.<name>] directory` block, a folder of `.eml` files, because an
`[imap.<name>]` server block is opened under `nvs test` like any other network door
(`rule:core-classes/imap`).

What a request or a run wrote crosses back into the test as a copy, because nothing crosses an
isolate boundary by reference (`rule:security/isolate-shares-nothing`).

**Declined:**
- **A fake for `Core\Process`.** A program that runs a command wraps it in its own class, and
  `Core\Test::double` replaces that class (`rule:testing/doubles`).
- **Running a `[[schedule]]` row from a test.** `spawn script` runs the script the row names, so a
  test reaches the same code without a scheduler.
