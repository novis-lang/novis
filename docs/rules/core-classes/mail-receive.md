`Core\Mail\Message::parse(string|bytes $raw): Core\Mail\Message` reads a raw RFC 5322 message once,
and every reader after it is a slot read. It never throws on a malformed message: a missing header is
`null` or an empty array, and a broken part is skipped, because a program reading a mailbox must reach
every message in it. `parse` takes a `string` too, as `Core\Mail\Attachment::fromBytes` does, because a
program may read an `.eml` file as one.

**The parser.** `mail-parser` 1.0 (Apache-2.0 OR MIT, fuzzed by its authors), wrapped so a program sees
Novis types only. It adds `encodify` and `hashify` by the same authors. The readers are `messageId()`,
`from(): ?Core\Mail\Address`, `to()`, `cc()` and `replyTo()` (`array<Core\Mail\Address>`),
`subject(): string`, `date(): ?Core\Time\DateTime`, `header(string $name): array<string>` (every value,
decoded), `inReplyTo()`, `references()`, `text()`, `html()`, `attachments():
array<Core\Mail\Attachment>` and `raw(): bytes`. `Core\Mail\Address` has `address()` and `name()`. A
received attachment is a `Core\Mail\Attachment`, so a program can forward one, and its `name()` is
`?string` for every attachment; `send` throws a catchable error for one with no name.

**What is read, and what is `null`.** A received address has one `@` with a local part and a domain,
and no space or control character; any other mailbox is left out of its list. `messageId()` is the
first id of its header. `date()` is `null` unless its year is 1900 to 3000, a numeric zone is `+hhmm`
with an hour under 24 and a minute under 60, and the calendar has that day and time. `mail-parser`
reduces an out-of-range zone, so the zone is read again from the field as written.

**Every value is tainted.** The bytes came from whoever sent the message. `html()` is a tainted string,
never `Core\Html\Markup`, so a page shows it through `Core\Html::sanitize`
(`rule:core-classes/html-sanitize`). An attachment's `bytes()` is `tainted bytes`.

**An attachment name is repaired, not refused.** It is the last path segment of the decoded name, with
control bytes removed, and `null` when nothing is left. `Core\Zip`'s traversal rule is the model, but a
received message was not written by the program, which must still reach its other parts.

**Bounds, counted before descending.** 16 levels of nesting and 1,000 parts are `mail-parser`'s own
`max_depth` and `max_parts`, and a part it flags `LIMIT_REACHED` is skipped. The wrapper reads only
the first 1,000 root header lines, and at most 1,000 addresses or ids from one header, because each
one read is a value many times the size of its bytes. A part past a bound is skipped and the message
keeps what came before. The raw bytes are already under the request's memory limit. What it spends: the parsed
message, per message read, freed with its last reference; an app that never parses pays nothing.

**The kind.** `kind(): Core\Mail\Message::Kind`, run once inside `parse`, is the first that matches of:
- `Bounce`: a `multipart/report; report-type=delivery-status` (RFC 3464) with a `failed` recipient.
- `Delay`: a delivery report with no `failed` recipient and at least one `delayed` one.
- `Complaint`: `report-type=feedback-report` (RFC 5965).
- `AutoReply`: `Auto-Submitted` present and not `no` (RFC 3834), or a row of the fixed table
  `X-Autoreply`, `X-Autorespond`, `Precedence: auto_reply`, `X-Auto-Response-Suppress` with `OOF`. A
  row is added only with a sample message that needs it.
- `Reply`: has `In-Reply-To`.
- `Other`.

A delivery report of `delivered` recipients only falls through the order. `message/global-delivery-status`
and `message/global-headers` (RFC 6533) are read as their ASCII forms are.

**What it answers.** `answers(): ?string` is the `Message-ID` of the mail this one is about: for a
bounce, a delay and a complaint, the one in the returned `text/rfc822-headers` or `message/rfc822`
part; otherwise `In-Reply-To`, then the last `References` entry. It is the other half of the id
`Core\Mail::send` returns (`rule:core-classes/mail-message`). `bounceTag(): ?string` is the `+tag` of
the first `Delivered-To`, else `X-Original-To`, else `To` address that has one, when it fits the tag
rule of `send`'s `bounceTag` (`rule:core-classes/mail-transport`); one predicate judges both halves.

**The report.** `report(): ?Core\Mail\Report` is set for a delivery report. `recipients():
array<Core\Mail\ReportRecipient>` gives each recipient's `address()` (`Final-Recipient`), `action()`
(`Core\Mail\ReportRecipient::Action`: `Failed`, `Delayed`, `Delivered`, `Relayed`, `Expanded`),
`status()` (the first word of `Status` when it is an RFC 3463 code such as `5.1.1`, else empty),
`isPermanent()` (status class 5) and `diagnostic(): ?string` (`Diagnostic-Code`). A report reads at
most 1,000 recipients, 1,000 lines per group with a folded line counted, and 4 KiB of one field. A
recipient with no `Final-Recipient` address or an `Action` RFC 3464 does not define is skipped. The
report's own first group is read like the others and gives no recipient, because it has no
`Final-Recipient`.
Both enums live in the class that returns them (`rule:enums/an-enum-lives-in-the-class-that-uses-it`).

**Declined:**
- **Novis as a receiving SMTP server on port 25.** Spam, abuse and open-relay risk, for a feature few
  apps need. A program reads a mailbox with `Core\Imap` (`rule:core-classes/imap`), or parses the raw
  message a provider's webhook posts.
- **A provider's own webhook JSON.** Each provider's shape is its own, and the raw message is the
  common one.
- **Guessing a bounce from a message that is not a delivery report.** A subject or a sender is
  ambiguous input (`rule:errors/ambiguous-input-refused`), and the large senders all write RFC 3464,
  so such a message is `Other`.
- **`html()` as `Core\Html\Markup`.** It would put a stranger's markup on a page unescaped.
