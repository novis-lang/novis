---
looper: 1
title: "a program reads a mailbox over IMAP, parses each message, and knows whether it is a bounce, an auto-reply or a reply to a mail it sent"
summary: "Reached when Core\\Mail\\Message::parse reads a raw message into tainted headers, parts and attachments, classifies it as a bounce, a delay, a complaint, an auto-reply, a reply or other with the Message-ID it answers and each bounced recipient's status, Core\\Imap reads, searches, fetches, moves, flags and deletes over an operator-named [imap.<name>] block or a directory, and the rules, the feature proofs and the website say so."
context: [docs/rules/core-classes/mail-message.md, docs/rules/core-classes/mail-transport.md, docs/rules/core-api/tier-roster.md, crates/nvs-stdlib/src/mail/mod.rs]
stages:
  - id: 1
    title: "Core\\Mail\\Message::parse reads a raw message into tainted headers, parts and attachments, within bounds"
    status: done
    context: [Cargo.toml, crates/nvs-stdlib/Cargo.toml, crates/nvs-stdlib/src/mail/parts.rs, crates/nvs-stdlib/src/zip.rs]
  - id: 2
    title: "a parsed message is classified, names the Message-ID it answers, and a delivery report gives each recipient's status"
    status: done
  - id: 3
    title: "Core\\Imap opens an operator-named block, lists folders, searches and fetches a parsed message"
    status: done
    context: [crates/nvs-config/src/tree.rs, crates/nvs-config/src/default.toml, crates/nvs-stdlib/src/mail/smtp.rs, crates/nvs-stdlib/src/mail/oauth.rs, crates/nvs-host/src/tls.rs]
  - id: 4
    title: "Core\\Imap moves, flags and deletes, and a directory block serves tests and local work"
    status: done
    context: [crates/nvs-config/src/tree.rs]
  - id: 5
    title: "the rules, the reference, the examples, the feature proofs and the website say so"
    status: open
    context: [website/src/content/docs/guides/how-to-use/sending-mail.mdx, website/src/data/changes.json, docs/agent/prose.md]
  - id: 6
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: parse
    stage: 1
    name: "parse reads headers, encoded words, the text and HTML bodies and attachments, every value is tainted, an attachment name is its last path segment, and nesting, part count and header count stop at their bounds"
    run: [cargo, test, --lib, "mail::"]
    tests: [parse_reads_the_headers_and_decodes_encoded_words, parse_reads_the_text_and_the_html_body, parse_reads_attachments_and_inline_parts, an_attachment_name_is_its_last_path_segment, a_name_with_nothing_left_is_null, parse_stops_at_the_nesting_bound, parse_stops_at_the_part_count_bound, a_message_with_no_headers_is_an_empty_message, parse_never_panics_on_the_fuzz_corpus]
    timeout: 1800
  - id: parse-taint
    stage: 1
    name: "every value a parsed message returns is tainted, and its HTML cannot reach a response without Core\\Html::sanitize"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/mail-message-parse-returns-tainted-values.nvst]
    timeout: 900
  - id: classify
    stage: 2
    name: "a delivery report is a bounce or a delay with each recipient's action, status and diagnostic, a feedback report is a complaint, an auto-reply is known by its headers, a reply by In-Reply-To, and answers() returns the original Message-ID"
    run: [cargo, test, --lib, "mail::"]
    tests: [a_failed_delivery_report_is_a_bounce, a_delayed_only_report_is_a_delay, each_recipient_has_its_action_status_and_diagnostic, a_5xx_status_is_permanent_and_a_4xx_is_not, a_bounce_answers_the_message_id_in_its_returned_headers, a_feedback_report_is_a_complaint, an_auto_submitted_message_is_an_auto_reply, each_auto_reply_header_in_the_table_is_recognized, a_message_with_in_reply_to_is_a_reply_and_answers_it, a_message_with_none_of_these_is_other, the_bounce_tag_is_read_from_delivered_to]
    timeout: 1800
  - id: imap-read
    stage: 3
    name: "Core\\Imap speaks implicit TLS and STARTTLS with PLAIN, LOGIN and XOAUTH2, lists folders in modified UTF-7, searches by its filters, fetches a parsed message, reports UIDVALIDITY, and a folder name cannot inject a command"
    run: [cargo, test, --lib, "imap::"]
    tests: [imap_speaks_tls_from_the_first_byte_on_993, imap_starttls_is_required_and_verified, imap_signs_in_with_plain_login_and_xoauth2, folders_are_listed_and_decoded_from_modified_utf7, search_sends_each_filter_and_returns_uids, fetch_returns_a_parsed_message_with_its_uid, uid_validity_is_returned, a_folder_name_is_sent_as_a_literal_or_quoted_string, a_message_over_the_request_ceiling_is_an_error_before_it_is_read, the_session_closes_when_the_request_ends]
    timeout: 1800
  - id: imap-write
    stage: 4
    name: "move uses MOVE or copy and expunge, a flag is set and cleared, delete expunges only its own uid, and a directory block does each on files"
    run: [cargo, test, --lib, "imap::"]
    tests: [move_uses_move_when_offered, move_falls_back_to_copy_and_uid_expunge, a_flag_is_set_and_cleared, delete_expunges_only_its_own_uid, a_directory_block_lists_its_subdirectories_as_folders, a_directory_block_moves_flags_and_deletes_files]
    timeout: 1800
  - id: imap-novis
    stage: 4
    name: "a Novis program reads a directory mailbox, classifies a bounce and moves it"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/imap-a-directory-mailbox-is-read-classified-and-moved.nvst]
    timeout: 900
  - id: proofs
    stage: 5
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, mail-receive]
    timeout: 1800
  - id: close
    stage: 6
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, mail-receive]
    timeout: 900
---

# a program reads a mailbox over IMAP, parses each message, and knows whether it is a bounce, an auto-reply or a reply to a mail it sent

## Why

A user's framework sends every mail from a noreply address and stores each delivery as a row. A relay
accepts most mail, and the receiving server rejects it later: that bounce, every out-of-office
answer and every reply land in the noreply mailbox. The framework wants a scheduled script that reads
that mailbox and marks each delivery bounced or answered. Novis reads no mailbox and parses no
message, so today it reads the mailbox through a provider's HTTP API.

`mail-complete` gives the sending half: `send` returns the `Message-ID`, `Auto-Submitted` stops most
out-of-office answers, and `bounceTag` puts the delivery into the bounce address. This goal is the
receiving half. The user chose all three pieces below.

There are many ways an app receives mail: polling a mailbox over IMAP, a provider's webhook that
posts the raw message, a provider's HTTP API that returns it, or a mail server that pipes each
message into a command. All of them end in the same raw message, so the parser serves every way, and
IMAP is the one way to fetch that every provider supports.

## Standing decisions

**Placement.** `rule:core-api/tier-roster`: `Core\Imap` is Native by test 3, as `Core\Mail`'s
transport is, because it waits on the outside world. `Core\Mail\Message::parse` reads hostile bytes
(test 5), and is Core despite that for `Core\Zip`'s reason: an attachment name's `../` is policy, and
policy must not be optional. The roster gains both lines in stage 5.

**Left out, declined into the rule's Declined list:** Novis as a receiving SMTP server on port 25
(spam, abuse and open-relay risk for a feature few apps need), POP3, JMAP, IMAP `IDLE` (a script
polls on a `[[schedule]]`; a held connection needs a socket isolate), a provider's own webhook JSON,
and guessing a bounce from a message that is not a delivery report.

**The parser (stage 1).** `mail-parser` 1.0.2 (Apache-2.0 OR MIT, fuzzed by its authors), wrapped
so the program sees Novis types only. It is not dependency-free: it adds `encodify` and `hashify`
(a build-time macro), both by its authors and Apache-2.0 OR MIT, over `memchr`, `simdutf8` and the
tree's `syn` 3. The workspace row says so. Its own `max_depth(16)` and `max_parts(1000)` are two of
the bounds, a part it flags `LIMIT_REACHED` is skipped, and the header bound is the wrapper's: it
reads only the first 1,000 root header lines (`receive.rs` `parse`).
- **Built in stage 1:** the parse runs once, and every reader is a slot read (`receive.rs`
  `SLOTS`). `parse` takes `string|bytes`, not only `bytes`, as `Attachment::fromBytes` does,
  because a program may read an `.eml` file as a string.
- **Built in stage 1:** received attachments are `Core\Mail\Attachment` values, so one can be
  forwarded. Its `name()` is now `?string` for every attachment (the bench `Mail-Attachment/name`
  writes `?? ''`), and `send` throws a catchable error for an attachment with no name. Its
  `bytes()` now returns `tainted bytes`: it was `Blob(Neutral)`, which compiled to plain `bytes`.
  That closes the backlog item about it.
- **Built in stage 1:** `Core\Mail\Address` gained `address()` and `name()`, both `tainted`, so
  `from()` and `to()` are readable.
- `Core\Mail\Message::parse(bytes $raw): Core\Mail\Message`. It never throws on a malformed
  message: a missing header is `null` or an empty array, and a broken part is skipped. A program
  reading a mailbox must be able to read every message in it.
- The readers: `messageId(): ?string`, `from(): ?Core\Mail\Address`, `to()`, `cc()` and
  `replyTo()` (`array<Core\Mail\Address>`), `subject(): string`, `date(): ?Core\Time\DateTime`,
  `header(string $name): array<string>` (every value, decoded), `inReplyTo(): ?string`,
  `references(): array<string>`, `text(): ?string`, `html(): ?string`, `attachments():
  array<Core\Mail\Attachment>`, `raw(): bytes`.
- **Every value is tainted.** `html()` is a tainted string, never `Core\Html\Markup`: a page shows
  it through `Core\Html::sanitize` (`crates/nvs-stdlib/src/html.rs:263`). The conformance case
  `mail-message-parse-returns-tainted-values` pins it.
- An attachment's `name()` is the last path segment of its decoded name, with control bytes removed,
  and `null` when nothing is left. `Core\Zip`'s traversal rule is the model; a received name is
  repaired, not refused, because the program reads a message it did not write and must still reach
  the other parts.
- **Bounds, counted before descending:** 16 levels of nesting, 1,000 parts, 1,000 header lines per
  part. A part past a bound is skipped and the message keeps what came before. The raw bytes are
  already within the request's memory ceiling. Memory: the parsed message, freed with its last
  reference.
- A fuzz target beside the existing ones (`crates/nvs-fuzz-checks/`), and the test
  `parse_never_panics_on_the_fuzz_corpus` runs its corpus.

**Classifying (stage 2).**
- `kind(): Core\Mail\Kind`, an enum: `Bounce` (a `multipart/report; report-type=delivery-status`,
  RFC 3464, with at least one `failed` recipient), `Delay` (a delivery report whose recipients are
  all `delayed`), `Complaint` (`report-type=feedback-report`, RFC 5965), `AutoReply`, `Reply` (has
  `In-Reply-To`), `Other`. Checked in that order, and the first match is the kind.
- `AutoReply`: `Auto-Submitted` present and not `no` (RFC 3834), or one of a fixed table of headers
  that common auto-responders write (`X-Autoreply`, `X-Autorespond`, `Precedence: auto_reply`,
  `X-Auto-Response-Suppress` with `OOF`). Each table row is one test. A row is added only with a
  sample message that needs it.
- `answers(): ?string`, the `Message-ID` of the mail this one is about: for a bounce, a delay and a
  complaint, the `Message-ID` in the returned original (`text/rfc822-headers` or `message/rfc822`
  part); otherwise `In-Reply-To`, then the last `References` entry.
- `report(): ?Core\Mail\Report`, for a delivery report: `recipients(): array<Core\Mail\ReportRecipient>`,
  each with `address()` (`Final-Recipient`), `action()` (`Core\Mail\ReportAction`: `Failed`,
  `Delayed`, `Delivered`, `Relayed`, `Expanded`), `status()` (`"5.1.1"`), `isPermanent()` (status
  class 5) and `diagnostic(): ?string` (`Diagnostic-Code`).
- `bounceTag(): ?string`: the `+tag` of the first `Delivered-To`, else `X-Original-To`, else `To`
  address that has one, when the tag fits `mail-complete`'s tag rule; otherwise `null`. It is the
  other half of `send`'s `bounceTag`.
- A bounce that is not a delivery report is `Other`. Guessing from a subject or a sender is declined:
  `rule:errors/ambiguous-input-refused`, and the large senders all write RFC 3464.
- **Built in stage 2:** the code is `crates/nvs-stdlib/src/mail/receive/classify.rs`, run once
  inside `parse`, and its result is four more slots of the message. The two enums are
  `Core\Mail\Message::Kind` and `Core\Mail\ReportRecipient::Action`, not `Core\Mail\Kind` and
  `Core\Mail\ReportAction`: `rule:enums/an-enum-lives-in-the-class-that-uses-it` puts an enum one
  class returns inside that class. `Delay` is a delivery report with no `failed` recipient and at
  least one `delayed` one, so a report that mixes `delayed` and `delivered` is a delay. A delivery
  report of `delivered` recipients only is not a bounce or a delay and falls through the order, so
  it is `AutoReply` when it has `Auto-Submitted`. A report reads at most 1,000 recipients
  (`classify::MAX_RECIPIENTS`) and 1,000 lines per group; a recipient with no `Final-Recipient`
  or an `Action` RFC 3464 does not define is skipped. `status()` is the first word of `Status`,
  empty when there is none. `message/global-delivery-status` and `message/global-headers` (RFC
  6533) are read as their ASCII forms are. `send`'s tag rule is one predicate,
  `smtp::is_bounce_tag`, that both halves call.

**`Core\Imap` (stages 3 and 4).** The shape is `[mail.<name>]`'s.
- `[imap.<name>]` in `crates/nvs-config/src/tree.rs` (beside `mail`, `:113`) and `default.toml`:
  `host`, `port` (default 993), `tls` (`implicit` default, `starttls`; no plaintext), `tls_ca_file`,
  `user` with `password` or `password_file`, or `mail-complete`'s `oauth_*` keys, `timeout`, and
  `directory` (stage 4). A grant `imap` lists the block names a program may open (`tree.rs:343`
  beside `mail`). The endpoint name is a `Qual::Sink`, as `Core\Mail`'s is.
- **Built in stage 3:** the grant is `[capabilities.imap] connect`, `Cap::ImapConnect`, named
  `connect` as `ldap.connect` is, because both open an operator-named block by name. The block is
  `ImapMailbox`, judged in `crates/nvs-config/src/imap.rs`. It reuses `mail::Tls` and `mail::Auth`.
  `tls = "none"` does not boot, and neither does half a credential. A block with no credential
  boots, as a mail block does, and `open` refuses it. A `directory` block boots with no other key,
  and its path is made absolute against the file that wrote it, as a `[storage]` root is. The
  default file's example block is `[imap.inbox]`.
- `Core\Imap::open(string $endpoint): Core\Imap\Mailbox`. `Mailbox` has `folders(): array<string>`
  and `folder(string $name): Core\Imap\Folder`. `Folder` has `uidValidity(): int`, `search({unseen,
  since, before, from, subject, limit}): array<int>` (UIDs, oldest first), `fetch(int $uid):
  Core\Mail\Message` (with `uid()`), and in stage 4 `move(int $uid, string $folder)`,
  `markSeen(int $uid)`, `markUnseen(int $uid)`, `flag(int $uid, string $flag)`, `unflag(int $uid,
  string $flag)` and `delete(int $uid)`.
- The protocol: `imap-codec` 1.x (MIT OR Apache-2.0), which encodes and decodes without doing IO,
  driven over `NvsTcp` and `NvsTls` as `mail/smtp.rs` drives SMTP. A folder name and every search
  string are encoded by the codec as a quoted string or a literal, so no value can end a command.
  Folder names are decoded from modified UTF-7.
- One connection per `open`, closed when the request or run ends (state lives as long as its
  request). No pool: a script polls, and a mailbox session holds a selected folder.
- `fetch` reads `RFC822.SIZE` first; a message larger than the request's remaining memory is an
  `IOError` before its body is read. `move` uses `MOVE` (RFC 6851) when offered, else `COPY` and
  `UID EXPUNGE` (RFC 4315) of that one UID. `delete` sets `\Deleted` and runs `UID EXPUNGE` of that
  one UID, never a plain `EXPUNGE`.
- **The directory block** (stage 4): `[imap.<name>] directory = "path"` reads a folder of folders
  instead of a server. Each subdirectory is a folder, each `<uid>.eml` file a message, flags live in
  `<uid>.flags` beside it, and `move` and `delete` rename and remove files. Under `fs.read` and
  `fs.write` for that path. A test and local development use it with no server, as a capture block
  does for sending. `uidValidity()` is 1.
- Under `nvs test`, a server block is opened like any other network door: a test uses a directory
  block, and `rule:testing/a-test-reads-its-logs-and-mail` gains one sentence saying so.
- **Built in stage 4, the server writes.** `imap-codec` 1.x has no `UID EXPUNGE`, so
  `session::uid_expunge` writes `<tag> UID EXPUNGE <uid>` itself: the line holds a tag and a
  number and nothing a program wrote. `delete` and a `move` without `MOVE` need `UIDPLUS`, and
  without it they are an `IOError` before any command is sent. A plain `EXPUNGE` would remove
  every `\Deleted` message, including ones another client marked. `UID STORE` is sent without
  `.SILENT`, so the server's `FETCH` reply shows the message exists. A `UID MOVE` with no
  `EXPUNGE` reply is "no message with that UID".
- **Built in stage 4, the flags.** `flag` and `unflag` take `\Answered`, `\Flagged`, `\Draft`,
  `\Seen` (in any case, sent in this case) or an IMAP atom keyword, as written (`imap::flag_of`).
  `\Deleted` is a `RuntimeError` that names `delete`, so only `delete` sets it and every
  `\Deleted` message this program marks is expunged at once. Any other `\` name and text that is
  not an atom are a `RuntimeError` too.
- **Built in stage 4, the directory block.** `imap/directory.rs`, behind `Held`'s `Backend` enum.
  `open` asks `fs.read` for the directory, and each of the six changes asks `fs.write` first
  (`writing_folder`). A folder is one directory name: no `/`, `\`, `:`, control character, `.` or
  `..` (a `RuntimeError`). Only `<uid>.eml` with no leading zero is a message. `since` and
  `before` read the `Date` header, because a test cannot set a file's arrival date, and a message
  with no `Date` matches neither. `from` and `subject` match the decoded header text, ignoring
  case. `move` gives the message the highest UID in the target plus one, and carries its
  `.flags` file. A freed highest UID can be given again; a directory block is for tests and local
  work, so that is accepted. The files are read with blocking calls for the same reason.

**Cost.** The parse and the classification run per message read, never on the request path of an
app that does not receive mail; a program that never calls them pays nothing. Two dependencies,
`mail-parser` and `imap-codec`, with no network or async code of their own.

**Rules.** A new fragment `docs/rules/core-classes/mail-receive.md` holds the parser, its bounds,
taint, the kinds, the report and § *Declined*. A new fragment `docs/rules/core-classes/imap.md`
holds the block, the grant, the API, the directory block and its Declined list.
`rule:core-api/tier-roster` gains the two placements.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is written to `### Backlog` with what blocked it.

**Lanes.** On the chain behind `mail-complete`, which it builds on: `crates/nvs-stdlib/src/mail/`,
`Core\Mail\Address`, `Core\Mail\Attachment`, the oauth module and the bounce tag. Gate: the file
`docs/goals/mail-complete.md` no longer exists. Stages 1 and 2 touch `mail/receive.rs`; stages 3 and
4 touch `crates/nvs-stdlib/src/imap/` and `crates/nvs-config/`, so stage 3 may run beside stage 2 in
a worktree.

## Stages

### 1. Core\Mail\Message::parse reads a raw message into tainted headers, parts and attachments, within bounds

Gate: `docs/goals/mail-complete.md` does not exist.

- [x] `Cargo.toml:1` and `crates/nvs-stdlib/Cargo.toml:1`: `mail-parser = "1"`.
- [x] `crates/nvs-stdlib/src/mail/receive.rs:1` (new): `Core\Mail\Message` and `parse`, the readers,
  taint on every text, per § *The parser*.
- [x] `crates/nvs-stdlib/src/mail/receive.rs:1`: attachment names, after the traversal check of
  `crates/nvs-stdlib/src/zip.rs:2160`; the three bounds.
- [x] `crates/nvs-fuzz-checks/`: a `mail_parse` target and a seed corpus of neutral messages.
- [x] `tests/conformance/core/mail-message-parse-returns-tainted-values.nvst:1` (new).
- [x] The nine tests of the check `parse`, then `cargo test --lib mail::` once.

### 2. a parsed message is classified, names the Message-ID it answers, and a delivery report gives each recipient's status

- [x] `crates/nvs-stdlib/src/mail/receive.rs:1`: `Core\Mail\Kind` and `kind()`, the auto-reply
  table, per § *Classifying*.
- [x] `crates/nvs-stdlib/src/mail/receive.rs:1`: `report()`, `Core\Mail\Report`,
  `Core\Mail\ReportRecipient`, `Core\Mail\ReportAction`, `answers()` and `bounceTag()`.
- [x] Neutral sample messages for each kind under `crates/nvs-stdlib/src/mail/samples/` (written by
  hand, never copied from a real mailbox).
- [x] The eleven tests of the check `classify`, then `cargo test --lib mail::` once.

### 3. Core\Imap opens an operator-named block, lists folders, searches and fetches a parsed message

- [x] `crates/nvs-config/src/tree.rs:113` and `:343`, `crates/nvs-config/src/default.toml:864`:
  `[imap.<name>]` and the grant `imap`, with their settings errors.
  - [x] `ImapMailbox`, `CapImap` and `Cap::ImapConnect` (`imap.connect`).
  - [x] `crates/nvs-config/src/imap.rs`: `canonicalize` and `validate`, with three tests.
  - [x] The three secret pairs, the defaults rows and `default.toml`'s blocks.
  - [x] The census tests, the secret fixture and the LSP's `NAMED`.
- [x] `Cargo.toml:1` and `crates/nvs-stdlib/Cargo.toml:1`: `imap-codec = "1"`.
- [x] `crates/nvs-stdlib/src/imap/mod.rs:1` and `session.rs:1` (new): the class rows, `open`, TLS,
  sign-in over `mail/oauth.rs`, `folders`, `folder`, `uidValidity`, `search`, `fetch`.
  - [x] `session.rs`: the protocol over `imap-codec`, TLS, `STARTTLS`, the three sign-ins,
    `folders`, `select`, `search`, `fetch` and modified UTF-7.
  - [x] `mod.rs`: the block read, the grant, `open`, `Mailbox`, `Folder` and their class rows.
  - [x] `oauth::grant_of` takes the block's family.
  - [x] `receive::message_of` gives `uid()`.
  - [x] The roster rows of each new class, and their tests run by name.
- [x] The ten tests of the check `imap-read` against a fake IMAP server in the test module, then
  `cargo test --lib imap::` once.
  - [x] The fake server in `imap/session/tests.rs`, and the eight tests that need only the session.
  - [x] `fetch_returns_a_parsed_message_with_its_uid` and
    `the_session_closes_when_the_request_ends`, over the class rows.
  - [x] `cargo test --lib imap::` once, green.

### 4. Core\Imap moves, flags and deletes, and a directory block serves tests and local work

- [x] `crates/nvs-stdlib/src/imap/session.rs:1`: `move`, `markSeen`, `markUnseen`, `flag`, `unflag`,
  `delete`, per § *`Core\Imap`*.
  - [x] `Session::store`, `move_to`, `delete` and `uid_expunge`.
  - [x] The six `Folder` rows, their cards and helpers in `imap/mod.rs`, and `flag_of`.
- [x] `crates/nvs-stdlib/src/imap/directory.rs:1` (new) and `crates/nvs-config/src/tree.rs:113`: the
  `directory` key and every `Folder` member over files.
  - [x] `Held` over a `Backend` enum, and `block_of` opens a directory under `fs.read`.
  - [x] `directory.rs` with every member, and each change under `fs.write`.
- [x] `tests/conformance/core/imap-a-directory-mailbox-is-read-classified-and-moved.nvst:1` (new),
  with its mailbox of neutral `.eml` files.
  - [x] Three more directory cases, so every `Mailbox` and `Folder` member has three.
  - [x] `conformance_coverage` run by name, green.
- [x] The six tests of the check `imap-write`, then `cargo test --lib imap::` once.

### 5. the rules, the reference, the examples, the feature proofs and the website say so

**Split:** the examples, the attacks and the benches go to one writing subagent once the rule
fragments are written, and the website section to a second; their files are disjoint. Each brief
names AGENTS.md § *Text an end user reads*.

- [x] `docs/rules/core-classes/mail-receive.md:1` (new), `docs/rules/core-classes/imap.md:1` (new),
  `docs/rules/core-api/tier-roster.md:20` and `docs/rules/testing/a-test-reads-its-logs-and-mail.md:7`,
  then `bun nv rules`.
- [x] The feature proofs `bun nv proofs --owed --goal mail-receive` names. The examples it will need
  include: a scheduled script that reads a noreply mailbox and marks each delivery bounced or
  answered by `answers()` and `bounceTag()`; parsing the raw message a webhook posts; showing a
  received HTML body through `Core\Html::sanitize`. Benches: parsing a typical message, and
  classifying a delivery report. Attacks: a message nested past the bound, a message with many
  parts, an attachment named `../../etc/passwd`, a folder name that tries to end the IMAP command, a
  tainted endpoint name.
- [x] `website/src/content/docs/guides/how-to-use/sending-mail.mdx`: a section "Reading bounces and
  replies", or a page of its own if the section passes the page's length.
- [x] `docs/reference/tools/20-config.md:1`: `[imap.<name>]` and the grant.
- [ ] `website/src/data/changes.json:1`: entries `new` `Core\Mail\Message::parse`, `new`
  `Core\Mail\Kind`, `new` `Core\Mail\Report`, `new` `Core\Imap`, `new` `[imap]` settings. Then
  `bun nv site --check stale`, `changes` and `review` green.
  - [x] The four `new` entries, and `site --check changes` green.
  - [ ] The newcomer review (`docs/agent/prose.md` § *A newcomer reads it*) of every text
    `site --review` lists, and each stamped; then `site --check stale`, `questions` and `review`.
- [ ] `bun nv proofs --owed --goal mail-receive` names nothing.

### 6. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Stages 1 to 4 are done, and stage 5 is done but for the newcomer review. Landed in stage 5:
- The rules `rule:core-classes/mail-receive` and `rule:core-classes/imap` (fragments under
  `docs/rules/core-classes/`, records under `data/rules/core-classes/`), the two roster placements,
  and the directory-block sentence in `rule:testing/a-test-reads-its-logs-and-mail`. `rules --check`
  is green.
- The feature proofs of all 46 features the goal reached: about.md, examples, attacks, benches and
  new `mail-message-*`, `mail-classify-*`, `mail-report-*` and `mail-address-*` cases. Three Rust
  tests gained `// covers:` lines. Every perf figure is recorded (`78a4ce9274`).
  `bun nv proofs --owed --goal mail-receive` now names only `reviewed`, 46 times.
- The website page `website/src/content/docs/guides/how-to-use/receiving-mail.mdx` (its own page:
  the sending page was 193 lines), with snippets under
  `website/snippets/guides/how-to-use/receiving-mail/`, linked from `sending-mail.mdx`, the guide
  index and `website/astro.config.ts`. `site --check snippets` is green.
- `docs/reference/tools/20-config.md`: the `[imap.<name>]` row, the `imap.connect` grant, the three
  secret keys. `changes.json`: four `new` entries, and `site --check changes` is green.

Expected red until the review: `site --check stale` (the new page's ids are never stamped),
`review` and `questions`, and the proofs gate's `reviewed` column. Not run: any test suite.

Without `--nvs target/debug/nvs.exe`, `bun nv proofs --bless` and `--run` build `--profile proof`
first, and two sessions building at once is a HANG on the build lock.

### Next

Finish stage 5's last item, then stage 6:
1. The newcomer review, per `docs/agent/prose.md` § *A newcomer reads it*: `bun nv site --review`,
   save the key list once to `.agent-tmp/`, batches of 20 to `reader` subagents, fix every finding,
   then stamp only the saved keys. Then `site --check stale`, `questions` (stamp each page with
   `MSYS_NO_PATHCONV=1 bun nv site --questions --stamp <url>`) and `review` green, and tick the item.
2. Stage 6 fixes the goal's own bugs the proof agents found (the first list under § *Backlog*),
   each with a test, before the gate's run.

### Backlog

Found by stage 5's proof agents in this goal's own code. Stage 6 fixes each, with a test:
- `crates/nvs-stdlib/src/mail/receive.rs:571`: `date()` never range-checks the offset from
  `mail-parser`: `+2400` reads as UTC and `+0199` as `+01:39`. It should be `null`. A 20-digit year
  reads as 9999-01-01.
- `receive.rs:604`: `Message-ID: <a@b> <c@d>` gives `c@d`, the last id; the first is meant.
- `receive.rs:550`: a received address is kept if not empty, so `<ada>` and `<ada lovelace@x>` pass.
- `classify.rs:611`: `Final-Recipient: rfc822; <ada@example.org> extra` keeps `> extra` in the
  address; `bare_address` (`:652`) would fix it.
- `classify.rs:564`: the doc says groups are read "after the first", and the code never skips the
  per-message group.
- `classify.rs:584`: a folded continuation line has no size bound and does not count toward the
  1,000 lines; a 1 MB `Diagnostic-Code` is read whole.
- `classify.rs:427`: `is_permanent` is `starts_with("5.")`, so `5.` is permanent and `5` is not.
- `imap/directory.rs:386`: a `Date` that `date()` reads as `null` still matches both `since` and
  `before` in a directory search (`tests/hostile/core/Imap-Folder/search/mailbox/INBOX/3.eml`).
- `imap/directory.rs:159` and `imap/mod.rs:969`: `flag` takes a keyword of any length, so one
  1 MB keyword pushes `<uid>.flags` past `FLAGS_CEILING` and every later flag call on that message
  throws. `flag_of` should bound a keyword.
- `imap/directory.rs:281`: on Windows `folder('INBOX.')` and `folder('INBOX ')` open `INBOX`; on
  Linux both are an `IOError`. `is_folder_name` should refuse a trailing dot or space.
- Unbounded but within the memory limit: addresses in one header (`receive.rs:536`) and ids in
  `References` (`:560`). Decide a bound or record why none.

Left by `mail-complete`, about sending. A session of this goal that touches the file takes each.

- No test sends a signed message through `send` and the fake relay: the five `dkim` tests call
  `dkim::sign` directly. A test over `smtp::tests::send_to` with a `dkim_key_file` block would pin
  the wiring.
- The DKIM key file is a private key, and `secret.rs`'s read advisory (another account can read
  the file) does not cover it yet: `crates/nvs-config/src/secret.rs` and `mail.rs` `canonicalize`.
- The same inline attachment passed twice (`attachments: [$img, $img]`) is sent as two parts
  with one Content-ID, which RFC 2045 forbids. `check_cids` in `crates/nvs-stdlib/src/mail/parts.rs`
  should throw. `tests/hostile/core/Mail-Attachment/inlineBytes/` step 4 records it as sent now.
- The unused-image error in `parts.rs` (near line 700) says "attach it with `fromFile` or
  `fromBytes`" for an image made with `inlineFile` too.
- `css-inline` rewrites a CSS value's double quotes as single quotes when it copies it into a
  `style` attribute (`tests/hostile/core/Mail-Preview/html/` step 5). Inlining cost grows with
  rules times elements, and only depth is capped; the author of the markup alone can trigger it.

Left by CI runs 45813627231 and 45829711007. None of them is in mail code. Stage 6 checks each
again.

- `serve::tests::a_waiting_request_holds_its_connection_and_its_isolate_on_the_heap`
  (`crates/nvs-cli/src/serve.rs:5555`) failed on Linux x86_64 and aarch64 with `(333, 0) idle and
  (686, 96) waiting` pages for 32 connections. That is 11.03 host pages per waiting request,
  against `REQUEST_HOST_STACK_PAGES = 11` (`serve.rs:5604`), whose doc records 9.1 on aarch64.
  It passes in WSL on x86_64 (`bun nv linux-test --bin nvs <name>`), and `serve.rs` has not
  changed since the run. Find which frame on the request path grew before touching the bound.
- `tests/conformance/core/a-channel-send-waits-only-once-its-bound-is-reached.nvst` printed `---`
  for `--w` once on macOS aarch64. It looks like a scheduling race in the case.
- `proofs`: `site shots` named `website/snippets/guides/how-to-use/sending-mail/files/logo.png`,
  which has no `.shot`, and the other debug-viewer shots. Re-shoot with `bun nv site --shoot`.
