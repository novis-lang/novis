---
looper: 1
title: "Core\\Mail\\Message::parse reads Outlook .msg files, winmail.dat, S/MIME signed mail and uuencoded files"
summary: "Reached when Core\\Mail\\Message::parse reads a .msg or .oft file into the same Message an .eml gives, replaces a winmail.dat attachment with the files and body inside it, opens an opaque S/MIME signed message, turns uuencoded blocks into attachments, never throws on a broken file, stops at its bounds, and the rule, the feature proofs, the help and the website say so."
context: [crates/nvs-stdlib/src/mail/receive.rs, docs/rules/core-classes/mail-receive.md]
stages:
  - id: 1
    title: the rules say how parse reads Outlook files, winmail.dat, S/MIME and uuencode
    status: open
    context: [docs/agent/doc-style.md]
  - id: 2
    title: "the MAPI reader: the compound file, property streams, code pages, compressed RTF and HTML inside RTF"
    status: open
    context: [Cargo.toml, crates/nvs-stdlib/Cargo.toml]
  - id: 3
    title: "parse reads a .msg file into a Core\\Mail\\Message"
    status: open
    context: [crates/nvs-stdlib/src/mail/receive/classify.rs, crates/nvs-stdlib/src/mail/parts.rs]
  - id: 4
    title: "an embedded message is a .msg attachment, a signed .msg is opened, and the bounds hold"
    status: open
    context: [crates/nvs-stdlib/src/fuzz.rs, crates/nvs-fuzz-checks/src/mail_parse.rs]
  - id: 5
    title: "a winmail.dat attachment is replaced by the files and the body inside it"
    status: open
  - id: 6
    title: "an opaque S/MIME signed message is opened, and uuencoded blocks are attachments"
    status: open
  - id: 7
    title: the feature proofs of parse cover every new form
    status: open
    context: [docs/agent/testing.md, docs/agent/prose.md]
  - id: 8
    title: the help, the reference and the website say which files parse reads
    status: open
    context: [docs/agent/prose.md, website/src/data/changes.json]
  - id: 9
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: mapi-reader
    stage: 2
    name: "the MAPI reader reads property streams in both string forms and every code page, decompresses RTF to the spec's vectors and takes HTML out of encapsulated RTF, and stops at a lying size"
    run: [cargo, test, --lib, mail::receive::outlook]
    tests: [a_unicode_and_an_ansi_property_stream_read_the_same_text, an_ansi_string_is_decoded_with_the_internet_code_page_then_the_message_code_page, a_property_larger_than_eight_bytes_is_read_from_its_own_stream, compressed_rtf_matches_the_spec_examples, compressed_rtf_with_a_bad_crc_or_a_lying_raw_size_gives_no_body, encapsulated_html_is_taken_out_of_rtf, rtf_without_fromhtml_gives_no_html, a_declared_stream_size_past_the_input_reads_no_further_than_the_input]
    timeout: 1800
  - id: msg-read
    stage: 3
    name: "a .msg file gives the headers, addresses, date, ids, bodies and attachments an .eml of the same mail gives"
    run: [cargo, test, --lib, mail::receive::outlook]
    tests: [a_received_msg_reads_its_transport_headers_first, a_draft_msg_reads_its_recipient_table_and_has_no_date, an_exchange_sender_is_read_by_its_smtp_address, an_exchange_sender_with_no_smtp_address_is_null, the_body_is_html_then_rtf_html_then_text, attachments_by_value_keep_name_type_and_content_id, an_attachment_by_reference_or_ole_is_left_out, a_compound_file_that_is_not_a_message_is_an_empty_message]
    timeout: 1800
  - id: msg-nested
    stage: 4
    name: "an embedded message is a .msg attachment that parse reads again, a signed .msg gives its signed content, and nesting stops at 16 levels"
    run: [cargo, test, --lib, mail::receive::outlook]
    tests: [an_embedded_message_is_a_msg_attachment_that_parses_again, a_multipart_signed_msg_gives_the_body_and_files_of_its_signed_content, embedded_messages_past_sixteen_levels_are_left_out, a_forwarded_eml_attachment_parses_again]
    timeout: 1800
  - id: tnef
    stage: 5
    name: "a winmail.dat attachment is replaced by its files, its body fills only what the MIME message lacks, and a broken one stays an attachment"
    run: [cargo, test, --lib, mail::receive::tnef]
    tests: [winmail_dat_is_replaced_by_the_files_inside_it, a_tnef_long_file_name_wins_over_the_short_title, a_tnef_body_fills_html_only_when_the_mime_message_has_none, a_tnef_attribute_with_a_bad_checksum_is_skipped, a_tnef_length_past_the_input_keeps_winmail_dat_as_it_was, a_tnef_message_inside_tnef_is_left_out]
    timeout: 1800
  - id: smime-uuencode
    stage: 6
    name: "an opaque signed message gives its content in BER and DER, an encrypted one stays an attachment, and uuencoded blocks become attachments only when every line checks out"
    run: [cargo, test, --lib, mail::receive]
    tests: [an_opaque_signed_message_gives_its_content, an_indefinite_length_ber_signed_message_gives_its_content, an_encrypted_message_stays_an_attachment, a_signed_message_inside_a_signed_message_counts_toward_the_depth, a_uuencoded_block_becomes_an_attachment_and_leaves_the_text, a_block_with_one_bad_line_stays_text, an_x_uuencode_part_is_decoded]
    timeout: 1800
  - id: uuencode-case
    stage: 6
    name: "a program reads a uuencoded file from a message as an attachment"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/mail-message-a-uuencoded-file-is-an-attachment.nvst]
    timeout: 1800
  - id: proofs
    stage: 7
    name: "every feature the goal's work reaches has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, outlook-messages]
    timeout: 1800
  - id: close
    stage: 9
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, outlook-messages]
    timeout: 900
---

# Core\Mail\Message::parse reads Outlook .msg files, winmail.dat, S/MIME signed mail and uuencoded files

## Why

`Core\Mail\Message::parse` reads an `.eml` file (`rule:core-classes/mail-receive`), but a web app
also gets mail in three forms it cannot read:

- **A `.msg` file.** A user drags a mail from classic Outlook into a browser upload, and the browser
  sends a `.msg`. Ticketing, CRM and document apps get these every day. Today `parse` gives an empty
  message for one.
- **A `winmail.dat` attachment.** Exchange wraps the real attachments and the formatted body in one
  TNEF file. Today `attachments()` returns that one file, and the files inside it are out of reach.
- **Opaque S/MIME signed mail and uuencoded files.** An opaque signed message has its whole body
  inside `smime.p7m`. `mail-parser` 1.0 decodes no uuencode at all: not inline `begin 644` blocks and
  not `Content-Transfer-Encoding: x-uuencode` (`mail-parser-1.0.2/src/decoders/mod.rs:258`).

`.msg` and TNEF use the same MAPI property model, so one reader serves both.

## Standing decisions

**The user's calls.** Single mail files only: `.msg`, `.oft`, TNEF inside an `.eml`, and the two
`.eml` gaps above. No container format: `.pst`, `.ost`, mbox and `.emlx` are declined (below). It
lives in `Core\Mail\Message::parse` itself, with no new class and no new reader.

**One entry point (my call).** `parse` reads a `.msg` when the bytes start with the compound file
signature `D0 CF 11 E0 A1 B1 1A E1`. An RFC 5322 message cannot start with those bytes, so this is
not a guess (`rule:errors/ambiguous-input-refused` holds). Every reader works on the result. `raw()`
returns the bytes as given. `uid()` is `null`. Detection is one 8-byte compare, so an app that reads
`.eml` pays nothing more.

**It never throws, as today.** A compound file that is not a message, such as a `.doc`, is an empty
message whose `raw()` is its bytes. A broken property, stream or attachment is skipped, and the
message keeps the rest.

**The compound file (my call).** The latest `cfb` crate release (MIT, `mdsteele/rust-cfb`), read from
an in-memory cursor and written for stage 4's rebuilt attachment. It goes into the workspace
`Cargo.toml` beside `mail-parser` (`Cargo.toml:1073`) with a comment in the same shape. Stage 2 first
checks its maintenance and that it reads and writes from memory. If it fails either, the fallback is
our own reader and writer of the [MS-CFB] subset: header, FAT, mini FAT, directory and streams, about
500 lines. A declared stream size is never trusted. Every read stops at the input's length.

**What `.msg` gives each reader.** Property ids are [MS-OXPROPS]. One code path reads both string
forms. `PT_UNICODE` (`0x001F`) is UTF-16LE. `PT_STRING8` (`0x001E`) is decoded with
`PidTagInternetCodepage` (`0x3FDE`), else `PidTagMessageCodepage` (`0x3FFD`), else windows-1252,
through `encoding_rs`, which is already in the tree.

- **Transport headers first.** When `PidTagTransportMessageHeaders` (`0x007D`) is present, the mail
  arrived over SMTP. Its headers are parsed with `mail-parser` exactly as an `.eml`'s are. They decide
  `header()`, `from()`, `to()`, `cc()`, `replyTo()`, `date()`, `messageId()`, `inReplyTo()`,
  `references()`, `kind()`, `answers()` and `bounceTag()`. MAPI properties fill only what the headers
  lack.
- **Without transport headers** (a draft, a sent item, an `.oft`):
  - `subject()` is `PidTagSubject` (`0x0037`).
  - `messageId()`, `inReplyTo()` and `references()` are `0x1035`, `0x1042` and `0x1039`.
  - `from()` is the sent-representing sender, as the `From` header is: `PidTagSentRepresentingSmtpAddress`
    (`0x5D02`), else `0x0065` when its type `0x0064` is `SMTP`, then the sender's `0x5D01`, else
    `0x0C1F` when `0x0C1E` is `SMTP`. The name is `0x0042`, else `0x0C1A`.
  - `to()` and `cc()` come from the recipient table, by `PidTagRecipientType` (`0x0C15`) 1 and 2.
    The address is `PidTagSmtpAddress` (`0x39FE`), else `0x3003` when `0x3002` is `SMTP`. The name
    is `0x3001`. Bcc is not read, because no reader returns it.
  - `date()` is `PidTagClientSubmitTime` (`0x0039`), else `PidTagMessageDeliveryTime` (`0x0E06`), in
    UTC, else `null`.
  - `header()` returns an empty array, and `replyTo()` is empty.
- **An Exchange address** (`/O=.../CN=...`) with no SMTP form is left out of its list, as any mailbox
  without one `@` is today. `from()` is then `null`.
- **The body.** `html()` is `PidTagHtml` (`0x1013`, decoded with the code page above). Otherwise it is
  the HTML taken from `PidTagRtfCompressed` (`0x1009`) when that RTF has `\fromhtml1` ([MS-OXRTFEX]).
  Otherwise it is `null`. `text()` is `PidTagBody` (`0x1000`), else `null`. RTF is never converted
  to text. Every value is `tainted`, as today.
- **Attachments.** By value (`PidTagAttachMethod` `0x3705` = 1) is read:
  - The bytes are `0x3701`.
  - The name is `PidTagAttachLongFilename` (`0x3707`), else `0x3704`. It is repaired by `file_name`
    (`receive.rs:749`).
  - The type is `PidTagAttachMimeTag` (`0x370E`) when `type_of` accepts it, else
    `crate::mime::detect_media` (`parts.rs:516`).
  - The content id is `0x3712`.
  - A hidden attachment (`0x7FFE`) is kept, because inline images are hidden.
  - By reference (2, 4, 7) is left out, because the file is not in the message. OLE (6) is left out,
    because it is an embedded Office object and not a file.
- **`kind()` of a `.msg`** is classified from its transport headers when it has them. A bounce saved
  from Outlook has no RFC 3464 parts, so it is `Other`. A program handles bounces through
  `Core\Imap`, where mail is always MIME. `classify::classified` (`classify.rs:490`) takes a root
  `mail-parser` message. Stage 3 gives it the parsed transport headers, or a header-only entry when
  there are none.
- **Named properties are not read.** No reader needs one.

**An embedded message (method 5) is a `.msg` attachment (my call).** Its sub-storage is written out as
a standalone compound file:
- The property stream header grows from 24 to 32 bytes.
- The root's `__nameid_version1.0` is copied in, so Outlook opens the file.

Its type is `application/vnd.ms-outlook`, and its name is its subject plus `.msg`, repaired. `parse`
reads its bytes again, as a forwarded `.eml` is read from its `message/rfc822` attachment. Stage 4
proves that the `.eml` side already works, or makes it work. Each level counts toward `MAX_DEPTH`
(`receive.rs:50`). A message within it past 16 levels is left out.

**A signed `.msg` (my call).** Message class `IPM.Note.SMIME.MultipartSigned` has an attachment
whose bytes are a whole `multipart/signed` MIME entity. It is parsed with `mail-parser`. Its text,
HTML and attachments replace the `.msg`'s own, which are often a placeholder. `IPM.Note.SMIME`
carries a PKCS #7 blob, and stage 6's unwrap reads it. The outer `.msg`'s headers and addresses stay.

**Compressed RTF ([MS-OXRTFCP]).** It has a 16-byte header with `compSize`, `rawSize`, the type (`LZFu`
or `MELA`) and a CRC. The 207-byte preset dictionary and a 4096-byte window are used. One 2-byte
token writes at most 17 bytes, so `rawSize` over 9 × `compSize` is a lie. That body is skipped.
Output stops at `rawSize`. A CRC mismatch skips the body. The decompressed RTF is freed as soon as its
HTML is taken out.

**TNEF ([MS-OXTNEF]) (my call).** A part is TNEF when its type is `application/ms-tnef`, or its name
is `winmail.dat`, and its bytes start with `78 9F 3E 22`:
- Its attachments replace it in `attachments()`. `attAttachRendData` starts each one.
- The bytes are `attAttachData`.
- The name is the long file name from `attAttachment`'s MAPI properties, else `attAttachTitle`,
  decoded with `attOemCodepage`.
- Its MAPI body (`0x1013`, `0x1009`, `0x1000`) fills `html()` only when the MIME message has none,
  and `text()` only when the MIME message has none.
- An attribute with a bad checksum is skipped.
- A length past the input stops the read, and the part stays as it was, because a half-read
  `winmail.dat` must not hide the original.
- A message inside TNEF (an attachment whose data is an `IMessage`) is left out.
- The MAPI properties in `attMSGProps` and `attAttachment` are decoded by the same property-value
  code as `.msg`, through a TNEF-specific framing.

**Opaque S/MIME (my call).** A part of type `application/pkcs7-mime` or `application/x-pkcs7-mime`,
with any `smime-type` or none, is checked:
- If its ContentInfo is `signedData` (`1.2.840.113549.1.7.2`) with `eContent`, that content is a MIME
  entity. It is parsed with `mail-parser`, and its text, HTML and attachments replace the part.
- At the root, they become the message's own. The outer headers stay.
- `envelopedData` (`1.2.840.113549.1.7.3`) stays an attachment, because no key is available.

The signature is not checked, and nothing says the message is signed. An unchecked "signed" is
misleading, and the content is `tainted` anyway. The decoder is our own bounded BER reader:
- It reads definite and indefinite lengths, and joins constructed OCTET STRING chunks.
- It reads only the path ContentInfo → SignedData → EncapsulatedContentInfo → eContent, about 150
  lines.
- The `der` crate in the tree is not used, because it refuses BER, and real signed mail uses BER.

Each unwrap counts toward `MAX_DEPTH`. A detached `multipart/signed` already reads its body, and its
`smime.p7s` stays an attachment, unchanged.

**uuencode (my call).**
- A block in a `text/plain` body is a line `begin <mode> <name>`, where the mode is 3 or 4 octal
  digits, then data lines, a zero-length line (`` ` `` or a space) and `end`.
- Every data line must check out. Its length character is 0 to 45. Its length is at least the 4 × ⌈n
  ÷ 3⌉ characters it needs. Every character is in `` ` `` and space to `_`.
- One bad line leaves the whole block as text, so ordinary text that starts with "begin" is never
  cut.
- A block that checks out is removed from `text()` and becomes an attachment. Its name is repaired by
  `file_name`, and its type is `detect_media`.
- A part with `Content-Transfer-Encoding` `x-uuencode`, `x-uue` or `uuencode` is decoded by the same
  code. The decoded size is under the input's.
- Blocks count toward `MAX_PARTS`. yEnc is declined: it is used only on Usenet.

**Bounds** (`rule:programs/memory-priority`). All are counted before descending, and each one's
overflow is skipped, never thrown:
- `MAX_DEPTH` 16 is shared by embedded `.msg`, signed content and nested MIME.
- `MAX_PARTS` 1,000 counts attachments plus uuencode blocks.
- `MAX_LIST` 1,000 recipients.
- One `.msg` reads at most 10,000 properties in one stream. A stream of 16-byte entries is bounded
  by its size anyway, and the count is a second guard.
- A compound file's sector chains are followed at most (input length ÷ sector size) steps, so a loop
  stops.

**What it costs.** Nothing for a program that never calls `parse`. Nothing more for an `.eml`
without TNEF, S/MIME or uuencode, beyond the 8-byte compare and one check per attachment's type.
Memory for a `.msg` or TNEF message is the raw bytes plus the decoded values: about 2 to 3 times the
file, per message, charged to the request and freed with the last reference. While it runs, the
decompressed RTF needs up to 9 times its stream. A rebuilt embedded message needs one more copy of
its bytes.

**Fixtures (my call).** No real user's file enters the tree (AGENTS.md § *The rules only this project
has*). The `.msg` and TNEF fixtures are built by a test-only builder in the tests module, from a list
of properties and attachments. It writes Unicode and ANSI forms. The `cfb` writer, or the fallback
writer, builds the container. Compressed RTF tests use the worked byte examples in [MS-OXRTFCP]
§ 3. The example and attack `files/` folders hold `.msg` files this builder wrote, under neutral
names (`Shop`, `example.com`). A sample written by real Outlook is in the backlog, for the user to
supply.

**Fuzzing.** The `mail_parse` target (`crates/nvs-fuzz-checks/src/mail_parse.rs:6`) already calls
`parse`, so no new target. Stage 4 adds seeds under `fuzz/seeds/mail_parse/`: a `.msg`, a `.msg` with
an embedded one, and compressed RTF. Stage 5 adds an `.eml` with TNEF. Stage 6 adds an opaque signed
`.eml` and a uuencoded one. `cargo test` runs every seed.

**Code layout.**
- `crates/nvs-stdlib/src/mail/receive/outlook.rs` and `outlook/` contain the compound file, the
  property streams, `rtf.rs`, the `.msg` mapping and its tests.
- `receive/tnef.rs` contains TNEF.
- `receive/smime.rs` contains the BER reader and the unwrap.
- `receive/uuencode.rs` contains uuencode.
- Each file has one `#[cfg(test)]` (the `tests/capability.rs` scan).
- `receive.rs`'s `parse` (`receive.rs:542`) and `nvs_core_mail_message_parse` (`receive.rs:842`)
  route to them.

**Declined, for the rules' Declined lists:**
- **`.pst`, `.ost`, mbox, `.emlx` and every other container.** The user's call: Novis parses one mail.
  A mailbox file can be many gigabytes. It does not fit a value read once per request, and it is
  migration work, not request work.
- **Checking an S/MIME signature, or a reader that says a message is signed.** It needs a trust store
  and revocation, and an unchecked claim misleads.
- **Decrypting S/MIME or PGP.** No key is available.
- **Converting RTF to text.** `PidTagBody` is there in practice, and an RTF reader is a large surface.
- **Named properties, calendar, contact and task fields of a `.msg`.** No reader returns them.
- **OLE and by-reference attachments.** They are not files in the message.
- **A message inside TNEF.** It is rare. Reading it would need TNEF inside TNEF.
- **Classifying a `.msg` bounce from its MAPI report class.** Bounces are read through `Core\Imap`,
  where they are MIME.
- **yEnc.**
- **A helper that rewrites `cid:` images in `html()`.** A received inline attachment already has
  `isInline()` and `src()`, so `Core\Str::replace` does it in three lines. Stage 7's example shows it.

**Lanes.** Stages 2 to 4 share `outlook/` and run in order. Stage 5 (`tnef.rs`) and stage 6
(`smime.rs`, `uuencode.rs`) touch disjoint files, need only stage 2's property-value code, and can
run as side goals once stage 2 is done. All three touch `receive.rs`'s `parse` in one place each.
Stage 7's proof files are disjoint and go to writing subagents.

**What stops it.** One attempt per item and two sessions per stage. A stage that does not close in two
sessions writes what holds it in the handoff, and the next session splits it.

## Stages

### 1. the rules say how parse reads Outlook files, winmail.dat, S/MIME and uuencode

- [ ] `docs/rules/core-classes/mail-outlook.md:1`: a new rule, written the way `docs/agent/doc-style.md`
  says, from the standing decisions about `.msg`, the embedded and signed `.msg`, compressed RTF and
  TNEF: what each reader gives, the bounds, what it costs, and its Declined list.
- [ ] `docs/rules/core-classes/mail-receive.md:1`: the first paragraph says `parse` also reads a `.msg`
  and points to the new rule. New paragraphs say how opaque S/MIME and uuencode are read. The bounds
  paragraph names the shared depth. The Declined list gains the container formats, signature checks,
  decryption and yEnc. Each paragraph is rewritten as a whole.
- [ ] `crates/nvs-stdlib/src/mail/receive.rs:1`: the module doc names the new forms and the new rule,
  and is rewritten as a whole (conventions.md § *A comment*).
- [ ] `bun nv rules --render`, then `bun nv rules --check` is green.

### 2. the MAPI reader: the compound file, property streams, code pages, compressed RTF and HTML inside RTF

**Split:** none, one session.

- [ ] `Cargo.toml:1073`: check `cfb`'s latest release and maintenance, and that it reads and writes in
  memory. Add it with its comment, or record the fallback here and write the reader instead.
- [ ] `crates/nvs-stdlib/src/mail/receive/outlook.rs:1`: open a compound file from bytes. Read the
  top-level, recipient and attachment property streams, with headers of 32, 8 and 8 bytes. Read each
  16-byte entry, and the `__substg1.0_<id><type>` stream of a value larger than 8 bytes. Decode
  `PT_UNICODE`, `PT_STRING8` with the code page order, `PT_BINARY`, `PT_SYSTIME`, `PT_LONG` and
  `PT_BOOLEAN`. The property-value decoding is a function that TNEF calls in stage 5. Every read
  stops at the input's length.
- [ ] `crates/nvs-stdlib/src/mail/receive/outlook/rtf.rs:1`: LZFu and MELA with the CRC and the
  `rawSize` limit. HTML is taken out of `\fromhtml1` RTF: the `\*\htmltag` groups give HTML,
  `\htmlrtf` groups are skipped, and `\par`, `\tab`, `\'hh` and `\uN` are read in the HTML parts.
- [ ] `crates/nvs-stdlib/src/mail/receive/outlook/tests.rs:1`: the fixture builder, and the tests the
  check `mapi-reader` names.

### 3. parse reads a .msg file into a Core\Mail\Message

**Split:** none, one session.

- [ ] `crates/nvs-stdlib/src/mail/receive.rs:842`: `nvs_core_mail_message_parse` routes bytes that
  start with the compound file signature to the `.msg` path. The result goes through the same
  `built` (`receive.rs:760`).
- [ ] `crates/nvs-stdlib/src/mail/receive/outlook.rs`: transport headers parsed with `mail-parser`
  first, then the MAPI readers as the standing decisions list them. Recipients, the date, the bodies
  and attachments by value.
- [ ] `crates/nvs-stdlib/src/mail/receive/classify.rs:490`: `classified` is reached for a `.msg`. It
  gets the transport headers' root, or a header-only entry when there are none.
- [ ] `crates/nvs-stdlib/src/mail/receive/outlook/tests.rs`: the tests the check `msg-read` names.

### 4. an embedded message is a .msg attachment, a signed .msg is opened, and the bounds hold

**Split:** none, one session.

- [ ] `crates/nvs-stdlib/src/mail/receive/outlook.rs`: an attachment with method 5 is rebuilt as a
  standalone compound file, with the 32-byte header and the copied `__nameid_version1.0`, and given
  as `application/vnd.ms-outlook`. Depth counts toward `MAX_DEPTH`.
- [ ] `crates/nvs-stdlib/src/mail/receive/outlook.rs`: `IPM.Note.SMIME.MultipartSigned` reads its
  attachment's MIME entity and takes its text, HTML and attachments.
- [ ] `crates/nvs-stdlib/src/mail/receive.rs:723`: a forwarded `.eml`'s `message/rfc822` attachment
  has the whole inner message as its bytes. A test proves it, and `received` changes only if it does
  not.
- [ ] `fuzz/seeds/mail_parse/`: the `.msg` seeds. `cargo test --lib
  every_saved_fuzz_input_passes_its_targets_check` (`crates/nvs-fuzz-checks/src/tests.rs:65`) runs
  them once.
- [ ] `crates/nvs-stdlib/src/mail/receive/outlook/tests.rs`: the tests the check `msg-nested` names.

### 5. a winmail.dat attachment is replaced by the files and the body inside it

**Split:** none, one session. It can run as a side goal once stage 2 is done.

- [ ] `crates/nvs-stdlib/src/mail/receive/tnef.rs:1`: the TNEF reader. The signature, attributes with
  their level, id, length and checksum, `attAttachRendData`, `attAttachTitle`, `attAttachData`,
  `attAttachment`, `attMSGProps` and `attOemCodepage`. MAPI values go through stage 2's decoder.
- [ ] `crates/nvs-stdlib/src/mail/receive.rs:612`: a TNEF part in the attachment list is replaced by
  its files. Its body fills `html()` and `text()` only where they are `null`.
- [ ] `fuzz/seeds/mail_parse/`: an `.eml` with a `winmail.dat`.
- [ ] `crates/nvs-stdlib/src/mail/receive/tnef.rs`: the tests the check `tnef` names.

### 6. an opaque S/MIME signed message is opened, and uuencoded blocks are attachments

**Split:** none, one session. It can run as a side goal once stage 2 is done.

- [ ] `crates/nvs-stdlib/src/mail/receive/smime.rs:1`: the bounded BER reader and the unwrap. It is
  reached from `parse` for a pkcs7 part, and from stage 4's `IPM.Note.SMIME`.
- [ ] `crates/nvs-stdlib/src/mail/receive/uuencode.rs:1`: inline blocks in the text body, and the
  `x-uuencode` transfer encoding. The text left after a block is cut out, and blocks count toward
  `MAX_PARTS`.
- [ ] `crates/nvs-stdlib/src/mail/receive.rs:602`: `text` and `attachments` go through both.
- [ ] `tests/conformance/core/mail-message-a-uuencoded-file-is-an-attachment.nvst:1`: a message with
  one good block and one block with a bad line. The check `uuencode-case` runs it.
- [ ] `fuzz/seeds/mail_parse/`: an opaque signed `.eml` and a uuencoded one.
- [ ] The tests the check `smime-uuencode` names, in each module's tests.

### 7. the feature proofs of parse cover every new form

**Split:** the example files and the attack go to two writing subagents side by side, and none of
them builds. Each gets the standing decisions, § *Text an end user reads* in AGENTS.md and the
existing `docs/examples/core/Mail-Message/parse/02-read-an-eml-file.nvs` as the model. Stage 3's
builder writes the `files/`. The session builds, runs `bun nv proofs --comments` over every new or
changed `.nvs`, then runs each example once.

- [ ] `docs/examples/core/Mail-Message/parse/04-read-a-file-dragged-from-outlook.nvs:1`: a `.msg` from
  an upload, its sender, subject and attachments.
- [ ] `docs/examples/core/Mail-Message/parse/05-the-files-inside-winmail-dat.nvs:1`: an `.eml` whose
  `winmail.dat` gives two files.
- [ ] `docs/examples/core/Mail-Message/parse/06-show-inline-images-on-a-page.nvs:1`: `html()` with each
  inline attachment's `src()` replaced by a URL, then `Core\Html::sanitize`.
- [ ] `docs/examples/core/Mail-Message/parse/about.md`: it names `.msg`, `.oft`, `winmail.dat`, signed
  mail and uuencode, what each gives, and that an Exchange address with no SMTP form is `null`. It
  stays under the about.md length limit.
- [ ] `tests/hostile/core/Mail-Message/parse/02-a-broken-outlook-file.nvs:1`: the attack reads crafted
  files from its `files/`. They are a compound file whose FAT chain loops, a stream that declares
  4 GiB, compressed RTF whose `rawSize` lies, embedded messages 5,000 deep, and a `winmail.dat` whose
  length points past its end. Each `parse` returns quickly with what it could read.
- [ ] `benches/members/core/Mail-Message/parse.nvs`: a `.msg` case beside the `.eml` one, with growth
  by attachment count.
- [ ] `bun nv proofs --owed --goal outlook-messages` names nothing, and the check `proofs` is green.
  Each feature it names is paid here, new or changed, listed above or not.

### 8. the help, the reference and the website say which files parse reads

- [ ] `crates/nvs-stdlib/src/mail/receive.rs:299`: `PARSE_DOC` says which files `parse` reads.
  `RAW_DOC` says `raw()` returns the bytes as given. `TEXT_DOC`, `HTML_DOC` and `ATTACHMENTS_DOC`
  say what an Outlook file, `winmail.dat` and a uuencoded block give.
- [ ] `website/src/data/changes.json`: `changed` `Core\Mail\Message::parse` (reads `.msg` and `.oft`
  files, `winmail.dat`, opaque signed mail and uuencoded files).
- [ ] Every handwritten website page about receiving mail is reread and stamped (`bun nv site
  --stamp`). An `Explore` subagent lists them first.
- [ ] `bun nv site --check stale` and `changes` are green.

### 9. the goal is verified once, and every failure is fixed

- [ ] The newcomer review, [prose.md](../agent/prose.md) § *A newcomer reads it*, over every page,
  `about.md` and card this goal wrote, and `bun nv site --check review` is green.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by the session that discussed the design with the user. Nothing is built yet. The user's
calls are marked in *Standing decisions*, and they are not reopened. The rest are my calls, built on
until the user picks another.

### Next

Stage 1 from its first item. The checks name tests and cases that do not exist yet, and they fail
until stages 2 to 7 write them.

### Backlog

- A `.msg` written by real Outlook (Unicode, ANSI, signed, with an embedded message) as an extra
  fixture, when the user supplies one under neutral names.
