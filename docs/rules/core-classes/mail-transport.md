A `[mail.<name>]` block is the whole transport: the server, how the session is secured and signed in,
the envelope, the connections kept, the DKIM key, and whether the block sends at all. A program
names the block and nothing else (`rule:core-classes/db-capabilities`'s shape), and every key below
is checked at boot as an `E0601` settings error, and again where it is read.

**TLS.** `tls` is `implicit` (TLS from the first byte, RFC 8314), `starttls` or `none`. Unset, it is
`implicit` on port 465, `starttls` when the block has a credential, and `none` otherwise. A
credential beside `none` is refused at boot, by the send, and where the password is written, so no
path sends one in the clear. `starttls` with no credential is the internal relay, and its TLS is
required and verified like any other. Every session is verified against `host`, and against the
compiled-in anchors or the block's `tls_ca_file`, which replaces them for that block alone and is
trust-checked as `[db]`'s is. A certificate from an unknown issuer is an `IOError` that names
`tls_ca_file` as the fix.

**Sign-in.** `auth` is `plain`, `login` or `xoauth2`; unset, `PLAIN` where offered, else `LOGIN`. Each
needs TLS. `xoauth2` needs `user` and refuses a password: the host fetches a token from
`oauth_token_url` (`https://` only) with `oauth_client_id`, `oauth_client_secret` or its `_file`, and
an optional `oauth_scope`. With `oauth_refresh_token` or its `_file`, it is the refresh-token grant;
without, the client-credentials grant. The fetch runs over the host's own HTTP/1.1 client and
compiled-in anchors, never `Core\Http`, because the URL is the operator's. It runs before the relay
is reached, so a refused token opens no SMTP connection and is an `IOError` that names the block and
the token server's status. The token is kept per block per process until 60 seconds before it
expires, replaced when the grant's values change, and dropped when the relay refuses it. An `oauth_*`
key on a block whose `auth` is not `xoauth2`, an `auth` with no credential, and a `tls_ca_file`
beside `tls = "none"` are refused, because each would be read and never used.

**The envelope.** `return_path` (default: `from`) is `MAIL FROM`, and `From:` stays `from`. The
`bounceTag` option sends `MAIL FROM:<local+tag@domain>`, 1 to 64 of `A-Z a-z 0-9 . _ -`; the program
chooses the tag, never the local part or the domain. A message over the server's advertised `SIZE`,
counted with its terminator, is an `IOError` naming both sizes before any `MAIL FROM`. A rejected
recipient ends the transaction with `RSET` before `DATA`, and the `IOError` names every rejected
address. `251` is an accepted recipient.

**The pool.** Over TLS, each connection pays a handshake, two `EHLO`s and `AUTH`, so a block keeps
its sessions in `nvs_runtime::pool` under `mail:<name>`, with `[db.<name>.pool]`'s four keys:
`max = 4` per core, `idle = 2`, `lifetime = "5m"` (under RFC 5321's server timeout) and `acquire =
"5s"`. `pool = false` opens one connection per message. A send releases its session the moment its
message is accepted. A kept session sends `RSET` first, one the server closed is replaced once, one
unused for 30 seconds is closed with `QUIT`, and one that failed in any way is closed. A session
belongs to one block. What it spends: at most `idle` sessions per block per core between sends and
`max` while sending, bounded by the configuration, never by traffic.

**DKIM.** A block with `dkim_selector` and `dkim_key_file` (both or neither) signs every message it
sends; `dkim_domain` defaults to the domain of `from`. The key is one PKCS#8 `PRIVATE KEY`, RSA of
2048 to 4096 bits or Ed25519, read and refused at boot, and read again by a send only when its file
changed. The signature is relaxed/relaxed over the message the composer wrote, with `h=` naming once
each present header of the fixed list From, To, Cc, Subject, Date, Message-ID, MIME-Version,
Content-Type, Reply-To, In-Reply-To, References, List-Unsubscribe and List-Unsubscribe-Post. No call
site names a key: the signing identity is the deployment's, like `from`.

**Capture.** `capture = true` makes a block keep each message and open no connection, as every block
does under `nvs test` (`rule:testing/a-test-reads-its-logs-and-mail`). A capture block needs `from`
and refuses every key only a connection reads; `from_name` and `return_path` stay. With recording on,
a kept message is a `mail` line of the debug stream
(`rule:testing/the-debug-stream-reaches-the-service-through-the-disk`), and the viewer's Mail page
shows it. A block that sends spools only its event, never its content, so a password-reset link
never reaches the disk. A captured message is not signed.

**Declined:**
- **Opportunistic TLS.** Verified, an internal relay stops working on a setting nobody wrote;
  unverified, it is a handshake nobody checked.
- **Sending to the accepted part of a recipient list.** A program that wants that sends one message
  per recipient.
- **The `SIZE=` parameter on `MAIL FROM`.** The check before it covers the case.
- **An idle time as a setting.** `idle` stays a count, as it is for `[db]`, and no case yet needs
  another time.
- **Fetching a token through `Core\Http`, or with the relay's `tls_ca_file`.** The token server is
  the operator's, and the relay's anchors are for the relay.
