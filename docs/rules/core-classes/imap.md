An `[imap.<name>]` block is a whole mailbox: the server, how the session is secured and signed in, or a
directory on disk. A program names the block and nothing else, in `[mail.<name>]`'s shape
(`rule:core-classes/mail-transport`), and the grant `[capabilities.imap] connect` lists the blocks it
may open. The endpoint name is a `Qual::Sink`, so a `tainted` name does not compile.

**The block.** `host`, `port` (default 993), `tls` (`implicit`, the default, or `starttls`),
`tls_ca_file`, `user` with `password` or `password_file`, or `[mail]`'s `auth = "xoauth2"` and its
`oauth_*` keys, and `timeout`. `tls = "none"` does not boot, so a password is never sent in the clear,
and neither does half a credential. A block with no credential boots, as a mail block does, and `open`
throws for it. A `directory` block boots with no other key, and its path is made absolute against the
file that wrote it, as a `[storage]` root is. Every key is checked at boot as an `E0601` settings error.

**The API.** `Core\Imap::open(string $endpoint): Core\Imap\Mailbox`. `Mailbox` has `folders():
array<string>` and `folder(string $name): Core\Imap\Folder`. `Folder` has `uidValidity(): int`,
`search({unseen, since, before, from, subject, limit}): array<int>` (UIDs, oldest first),
`fetch(int $uid): Core\Mail\Message` with its `uid()` (`rule:core-classes/mail-receive`), `move(int
$uid, string $folder)`, `markSeen`, `markUnseen`, `flag(int $uid, string $flag)`, `unflag` and
`delete(int $uid)`. `search` returns a list, so its keys are positions and not tainted.

**The protocol.** `imap-codec` 1.x (MIT OR Apache-2.0) encodes and decodes with no IO of its own, driven
over the host's TCP and TLS as `Core\Mail` drives SMTP. A folder name and every search string are
encoded as a quoted string or a literal, so no value ends a command. Folder names are decoded from
modified UTF-7. One connection per `open`, closed when the request or run ends; there is no pool,
because a script polls and a session holds a selected folder. `fetch` reads `RFC822.SIZE` first, and a
message larger than the request's remaining memory is an `IOError` before its body is read.

**The writes.** `move` uses `MOVE` (RFC 6851) when offered, else `COPY` and `UID EXPUNGE` (RFC 4315)
of that one UID. `delete` sets `\Deleted` and runs `UID EXPUNGE` of that one UID, never a plain
`EXPUNGE`, which would remove every message another client marked. Both need `UIDPLUS`, and without it
they are an `IOError` before any command is sent. `imap-codec` has no `UID EXPUNGE`, so the host writes
`<tag> UID EXPUNGE <uid>` itself: the line has a tag and a number and nothing a program wrote. `UID
STORE` is sent without `.SILENT`, so the server's reply shows the message exists, and a `UID MOVE` with
no `EXPUNGE` reply is "no message with that UID". `flag` and `unflag` take `\Answered`, `\Flagged`,
`\Draft`, `\Seen` (in any case) or an IMAP atom keyword of at most 255 bytes. `\Deleted` is a `RuntimeError` that names
`delete`, so every `\Deleted` message this program marks is expunged at once; any other `\` name and
text that is not an atom throw too.

**The directory block.** `directory = "path"` reads a folder of folders instead of a server: each
subdirectory is a folder, each `<uid>.eml` (no leading zero) a message, and its flags are in
`<uid>.flags` beside it. `open` needs `fs.read` for the directory, and each change needs `fs.write`
first. A folder is one directory name: no `/`, `\`, `:`, control character, `.` or `..`, and no dot or
space at its end, which Windows removes when it opens a path. `since` and `before` read the `Date`
header, because a test cannot set a file's arrival date, and a message whose `Date` is `null` matches
neither. A flag that would push `<uid>.flags` past 64 KiB is an `IOError`, and nothing is written. `move` gives the
message the highest UID in the target plus one and carries its flags, and a freed highest UID can be
given again. `uidValidity()` is 1. The files are read with blocking calls, because the block is for
tests and local work (`rule:testing/a-test-reads-its-logs-and-mail`).

What it spends: one connection and one parsed message at a time per `open`, per request or run, freed
when it ends. An app that never opens a block pays nothing.

**Declined:**
- **POP3 and JMAP.** IMAP is what every provider offers, and one protocol is simpler.
- **IMAP `IDLE`.** A script polls on a `[[schedule]]`; a held connection would need a socket isolate.
- **A pool of sessions.** A session has a selected folder, and a polling script opens one per run.
- **A plain `EXPUNGE` or a `\Deleted` flag through `flag`.** Either removes messages the program did
  not name.
- **Plaintext IMAP.** A password is never sent without TLS.
