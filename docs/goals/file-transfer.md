---
looper: 1
title: "a program reads and writes files on FTP, FTPS and SFTP servers"
summary: "Reached when Core\\FileTransfer::open opens an operator's [file_transfer.<name>] block over SFTP, FTPS or anonymous FTP, or over a local directory, and its connection lists, walks, reads as bytes or as chunks, writes atomically from any source, appends, moves, removes, makes and removes directories, and sets permissions and modified times; SFTP runs over a client in Rust with no C and no second event loop, verifies every host key against a pin or a known_hosts file, and signs in with a password, keyboard-interactive or a key; FTPS verifies its certificate, reuses its TLS session on every data connection and never connects to the address a PASV reply names; connections are pooled per block and reset before reuse; every path is judged against the block's root; and the rules, the feature proofs, the interop tests against OpenSSH and vsftpd and the website say so."
context: [crates/nvs-stdlib/src/imap/mod.rs, crates/nvs-config/src/imap.rs, docs/rules/core-classes/imap.md, docs/rules/core-api/tier-roster.md, crates/nvs-host/src/net.rs, crates/nvs-host/src/tls.rs]
stages:
  - id: 1
    title: the rules say what Core\FileTransfer is, and the roster no longer declines FTP and SSH
    status: open
    context: [docs/rules/core-classes/imap.md, docs/rules/core-classes/mail-transport.md, docs/rules/security/one-tls-client.md, docs/rules/security/net-address-policy.md, docs/rules/programs/framework-core-half.md, crates/nvs-stdlib/src/storage.rs, docs/agent/doc-style.md]
  - id: 2
    title: "nvs.toml reads [file_transfer.<name>] blocks and the file_transfer.connect grant"
    status: open
    context: [crates/nvs-config/src/imap.rs, crates/nvs-config/src/capability.rs, crates/nvs-config/src/secret.rs, crates/nvs-config/src/lib.rs]
  - id: 3
    title: "Core\\FileTransfer, its connection and its entries work over a directory block"
    status: open
    context: [crates/nvs-stdlib/src/imap/mod.rs, crates/nvs-stdlib/src/imap/directory.rs, crates/nvs-stdlib/src/io.rs, crates/nvs-stdlib/src/registry.rs, crates/nvs-stdlib/src/byte_source.rs]
  - id: 4
    title: every member works over FTPS and anonymous FTP
    status: open
    context: [crates/nvs-stdlib/src/mail/smtp.rs, crates/nvs-host/src/net.rs, crates/nvs-host/src/tls.rs, crates/nvs-stdlib/src/imap/session/tests.rs]
  - id: 5
    title: "nvs-ssh speaks the SSH transport: key exchange, ciphers, host keys and rekeying"
    status: open
    context: [Cargo.toml, deny.toml, crates/nvs-host/src/net.rs]
  - id: 6
    title: nvs-ssh signs in with a password, keyboard-interactive or a key, and opens the SFTP subsystem
    status: open
    context: [Cargo.toml, crates/nvs-config/src/secret.rs]
  - id: 7
    title: every member works over SFTP, with pipelined reads and writes
    status: open
    context: [crates/nvs-stdlib/src/byte_source.rs, crates/nvs-runtime/src/ctx/held.rs]
  - id: 8
    title: interop tests run against OpenSSH and vsftpd on Linux, and fuzz targets cover every parser
    status: open
    context: [.github/workflows/ci.yml, fuzz/Cargo.toml, docs/agent/testing.md]
  - id: 9
    title: connections are pooled per block, and nvs prints a server's host key for the operator to pin
    status: open
    context: [crates/nvs-runtime/src/pool.rs, crates/nvs-stdlib/src/mail/pool.rs, crates/nvs-cli/src/main.rs]
  - id: 10
    title: the feature proofs of every new member
    status: open
    context: [docs/agent/testing.md, docs/agent/prose.md]
  - id: 11
    title: the help, the reference and the website say how a program transfers files
    status: open
    context: [docs/agent/prose.md, website/src/data/changes.json, website/src/content/docs/guides/how-to-use/receiving-mail.mdx]
  - id: 12
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: config
    stage: 2
    name: "a [file_transfer.<name>] block boots with each protocol, and each refused shape fails at boot with a message naming the key"
    run: [cargo, test, --test, file_transfer_config]
    timeout: 1800
  - id: directory-round-trip
    stage: 3
    name: "every member of a connection works over a directory block, and a file written reads back"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/file-transfer-a-directory-block-runs-every-member.nvst]
    timeout: 1800
  - id: root
    stage: 3
    name: "a path that leaves the block's root, or holds a NUL, CR or LF, throws before anything is sent"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/file-transfer-a-path-that-leaves-the-root-throws.nvst]
    timeout: 1800
  - id: ftps
    stage: 4
    name: "every member works over explicit and implicit FTPS and anonymous FTP against the in-process server, and a PASV address is never connected to"
    run: [cargo, test, --lib, file_transfer::ftp]
    timeout: 1800
  - id: ssh-transport
    stage: 5
    name: "nvs-ssh completes every key exchange and cipher it offers against the test server, refuses an unpinned host key, and rekeys"
    run: [cargo, test, --test, ssh_transport]
    timeout: 1800
  - id: ssh-auth
    stage: 6
    name: "nvs-ssh signs in with a password, keyboard-interactive and each key type, encrypted keys included, and opens the SFTP subsystem"
    run: [cargo, test, --test, ssh_auth]
    timeout: 1800
  - id: sftp
    stage: 7
    name: "every member works over SFTP against the test server, and a 16 MiB read takes the counted number of round trips"
    run: [cargo, test, --lib, file_transfer::sftp]
    timeout: 1800
  - id: interop
    stage: 8
    name: "every member works against a real OpenSSH sshd and a real vsftpd on Linux"
    run: [bun, nv, linux-test, --test, file_transfer_interop]
    timeout: 3600
  - id: pool
    stage: 9
    name: "a pooled connection is reused only after its handles are closed and its state is reset, and a request never sees another's"
    run: [cargo, test, --lib, file_transfer::pool]
    timeout: 1800
  - id: proofs
    stage: 10
    name: "every feature the goal's work reaches has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, file-transfer]
    timeout: 600
  - id: close
    stage: 12
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, file-transfer]
    timeout: 900
---

# a program reads and writes files on FTP, FTPS and SFTP servers

## Why

`rule:core-api/tier-roster` declines `ftp` and `ssh2` "by test 1 and a thin audience"
(`docs/rules/core-api/tier-roster.md:61`). The user reverses that: a web application exchanges files
with partners, banks, printers, payment providers and legacy systems over SFTP and FTPS every day, and
"run the `sftp` binary through `Core\Process`" is not an answer for a web server.

The roster's reason was also wrong in one place. It says "`Core\Storage` and the outbound address
policy cover FTP", but `Core\Storage` is local disk only and has no backend interface
(`crates/nvs-stdlib/src/storage.rs:5`, `rule:programs/framework-core-half`). Today a program has no way
to reach an FTP server at all.

The user's ask is "full FTP and SFTP: everything a web application or service actually needs". This
goal settles that scope below, and builds it the way `Core\Imap` is built: an operator's block in
`nvs.toml`, a capability grant, the host's own reactor and TLS client, one connection object, and a
directory backend for tests and examples.

## Standing decisions

**FTP and SFTP are in `Core`, Native (the user's call: reverse the decline).** The roster places
`Core\FileTransfer` with `Core\Imap` and `Core\Mail`'s transport: Native by test 3, because it waits on
the outside world, over the runtime's own reactor (`crates/nvs-host/src/net.rs:246`) and its one TLS
client (`rule:security/one-tls-client`). `soap` stays declined. `Core\Storage` stays local disk, and
the roster's sentence about it is rewritten to the truth.

**One API for every protocol, chosen by the operator.** A program names a block, never a protocol:
`Core\FileTransfer::open('partner')` returns a `Core\FileTransfer\Connection`, and the block's
`protocol` says whether that is SFTP, FTPS or a local directory. The same code runs against a
directory in tests and against the partner's server in production. This is
`rule:core-classes/net-one-api-three-transports`'s shape. The class name and the block name are
checked with `bun nv names --check` in stage 1; if it names a better one, the session takes it and
renames every use in this file.

**The block.** `[file_transfer.<name>]`, modelled on `[imap.<name>]` (`crates/nvs-config/src/imap.rs`):

| Key | Meaning |
|---|---|
| `protocol` | `sftp`, `ftps` or `ftp`. Unset with `directory` set, the block is a directory. |
| `host`, `port` | Port defaults: 22 for `sftp`, 21 for `ftps` and `ftp`, 990 for `ftps` with `tls = "implicit"`. |
| `tls` | FTP only: `explicit` (AUTH TLS, the default), `implicit` or `none`. |
| `user` | Unset on `ftp`/`ftps`, the sign-in is `anonymous`. Required on `sftp`. |
| `password`, `password_file` | A secret, as `[imap]` reads it (`crates/nvs-config/src/secret.rs`). |
| `key_file`, `key_passphrase_file` | SFTP only: an OpenSSH or PKCS#8 private key, and its passphrase. |
| `host_key` | SFTP only: one or more `SHA256:` fingerprints the server's host key must match. |
| `known_hosts_file` | SFTP only: an OpenSSH `known_hosts` file, read instead of `host_key`. |
| `tls_ca_file` | FTPS only: replaces the trust anchors for this block, as `[imap]`'s does. |
| `tls_pin`, `tls_verify_host`, `tls_verify`, `tls_min_version` | FTPS only: the relaxations every client block takes, each under its `[capabilities.tls]` grant, which goal `tls-relax-every-client` builds. The control and data connections both go through `NvsTls::over_policy` with the block's policy. |
| `root` | The remote folder every path is relative to. Unset, the login folder. |
| `directory` | A local folder that replaces the server, for tests, examples and development. |
| `timeout` | Idle time per operation, as `[imap]`'s `timeout`. |
| `pool`, and the pool bounds | As `[mail.<name>]`'s (`nvs_config::mail::POOL_DEFAULTS`). |

**What fails at boot**, each with a message naming the key, as `[imap]` and `[mail]` do:

- an `sftp` block with neither `host_key` nor `known_hosts_file`. There is no trust on first use: a
  first connection that trusts whatever key it sees is the one an attacker in the middle answers;
- `tls = "none"` beside a `password` or `password_file`. This is `rule:core-classes/mail-transport`'s
  rule for SMTP: plain FTP is allowed only for an anonymous sign-in, so no password ever crosses the
  network in clear text. A program that downloads public files from an anonymous FTP server works;
  one that signs in to a plain-FTP server does not, and the message says to ask for FTPS or SFTP;
- an SFTP key, `host_key` or `known_hosts_file` on an FTP block, or `tls` on an SFTP block, since each
  would be read and never used;
- `directory` beside `host`.

**The grant.** `[capabilities.file_transfer] connect = ["partner"]` lists the blocks a program may
open, as `[capabilities.imap] connect` does (`crates/nvs-config/src/capability.rs:201`). The host is
written by the operator, so it is not judged by `rule:security/net-address-policy`, exactly as an
`[imap]` host is not (`crates/nvs-stdlib/src/imap/mod.rs:918`).

**The connection's members.** The names follow `Core\IO`'s (`crates/nvs-stdlib/src/io.rs:100`-`:331`),
so a programmer who knows one knows the other. Every path is a `CoreTy::Path(Qual::Sink)`, as every
`Core\IO` path is, so a `tainted` value from a request does not compile until the program cleans it.

- `list(string $folder = ''): Core\FileTransfer\Listing` gives one `Entry` per item in the folder,
  without `.` and `..`, as the server sends them. It is an iterator, as `Core\IO\Walk` is in the
  `file-scan` goal, and reads the listing while the loop runs.
- `walk(string $folder = ''): Core\FileTransfer\Walk` gives every entry below the folder, depth-first,
  siblings in name order, reading one folder per step, exactly as `file-scan`'s `Core\IO::walk` does.
  A link is reported and never followed.
- `exists(string $path): bool` and `metadata(string $path): Core\FileTransfer\Entry`; `metadata`
  throws `IOError` for a path that does not exist.
- `readBytes(string $path): tainted bytes` returns the whole file. It asks the server for the size
  first, and a file larger than the request's remaining memory throws `IOError` before a byte is
  read, as `Core\Imap`'s `fetch` does (`docs/rules/core-classes/imap.md:24`).
- `readStream(string $path, uint $offset = 0): Core\Bytes::Chunks` gives the file as tainted chunks of
  at most 64 KiB, starting at `$offset`, so a download that was cut off resumes. It feeds
  `Core\IO::writeStream`, `Core\Response::startStream`, `Core\Zip\Writer::add` and `Core\Hash::of`.
- `write(string $path, Core\Bytes::Source|Core\Bytes::Chunks $content, bool $atomic = true): uint`
  writes from every source `rule:core-api/a-member-takes-the-source-the-program-holds` names, a
  `Core\IO\File` among them, and returns the bytes written. With `$atomic`, it writes
  `<name>.<16 hex>.part` in the same folder and renames it over `$path` when the last byte is
  acknowledged, so a partner polling the folder never picks up half a file. A failed write removes the
  part file. `$atomic = false` writes `$path` directly, for a server that forbids renames.
- `append(string $path, Core\Bytes::Source|Core\Bytes::Chunks $content): uint` writes at the end of
  the file, so an upload that was cut off resumes after `metadata($path)->size()`.
- `move(string $from, string $to): void` renames and replaces `$to` if it exists: SFTP uses
  `posix-rename@openssh.com` where offered, else removes `$to` first; FTP uses `RNFR`/`RNTO`.
- `remove(string $path): void`, `makeDirectory(string $path): void` and
  `removeDirectory(string $path): void` behave as `Core\IO`'s members of the same name do, recursion
  included, and stage 3 copies their exact semantics from `io.rs` into the rule.
- `setPermissions(string $path, uint $mode): void` (SFTP `SETSTAT`, FTP `SITE CHMOD`) and
  `setModifiedAt(string $path, Core\Time\Instant $at): void` (SFTP `SETSTAT`, FTP `MFMT`). A server
  that does not support one throws `IOError` naming the command.
- `close(): void` ends the connection early. Otherwise it ends with its request or run.

**`Core\FileTransfer\Entry`** has `path()`, `relativePath()`, `name()`, `isFile()`, `isDirectory()` and
`isLink()` with the meaning `file-scan` gives `Core\IO\Entry`, plus `size(): ?uint`,
`modifiedAt(): ?Core\Time\Instant` and `permissions(): ?uint`. A remote listing already carries those
three (MLSD facts, SFTP attributes), so they cost nothing extra, unlike on a local disk. Each is `null`
when the server does not send it. Every string is `tainted`: the server chose it.

**Declined**, each named so it is not re-proposed:

- **Plain FTP with a password**, for the reason above. Anonymous plain FTP is allowed.
- **Active-mode FTP (`PORT`/`EPRT`).** The server would connect in to the program, which needs an
  open inbound port and breaks behind NAT. Passive mode reaches every server a web host can reach.
- **ASCII transfer mode.** Every transfer is `TYPE I`. Changing line endings corrupts binary files,
  and a program converts text itself.
- **FXP**, a server-to-server copy through the program, and remote `copy` in general: SFTP v3 and FTP
  have no portable copy command.
- **SCP and running commands over SSH.** The SSH client opens the `sftp` subsystem and nothing else:
  no `exec`, no shell, no port forwarding, no agent forwarding. `Core\Process` runs a real `ssh` for
  the rest. OpenSSH itself deprecates the SCP protocol.
- **An SSH agent.** A key comes from a secret file only (`rule:security/secret-has-no-ambient-source`).
- **Trust on first use**, for the reason above. Stage 9's command prints the key for the operator to
  pin.
- **FTPS client certificates, proxies (SOCKS, HTTP CONNECT) and FTP over HTTP.** Nobody has asked for
  one. Each is additive later, and no block key is reserved for it.
- **SSH compression, the CBC and CTR ciphers, `hmac-sha1`, `ssh-rsa` (SHA-1) signatures, `ssh-dss`,
  and the Diffie-Hellman groups.** Each is weak, or needs primitives `ring` does not expose. What is
  offered (below) covers OpenSSH 6.5 and later, AWS Transfer Family, Azure Blob SFTP and Windows'
  OpenSSH server.
- **The post-quantum hybrid key exchanges** (`mlkem768x25519-sha256`, `sntrup761x25519-sha512`).
  `ring` has no ML-KEM. Every server that offers them also offers `curve25519-sha256`. When the
  dependency gains ML-KEM, it is added first in the list and no block changes.
- **A `Core\Storage` disk on a remote server.** `rule:programs/framework-core-half` keeps remote
  backends out of `Core\Storage`. A program that wants both writes to the connection.

**SSH is written in Rust in a new crate, `crates/nvs-ssh`, with no C and no async runtime.**

- `russh` needs a tokio runtime, which `rule:concurrency/one-scheduler` declines, for the reason
  `Cargo.toml:1151` declines `sqlx` and `tokio-postgres`. `ssh2` binds libssh2, a C library with its
  own history of memory-safety advisories, and needs a raw socket the reactor does not own. Both are
  declined, in the crate's `Cargo.toml` comment as `imap-codec`'s choice is explained at
  `Cargo.toml:1085`.
- `nvs-ssh` is sans-I/O: it turns bytes in into bytes out and events, and `nvs-stdlib` drives it over
  `NvsTcp`, as `imap-codec` is driven (`crates/nvs-stdlib/src/imap/session.rs:1`). It has
  `unsafe_code = "forbid"` from the workspace and is not on `UNSAFE_CRATES`.
- **Every primitive comes from `ring` 0.17**, which rustls already links: X25519 and P-256/P-384 ECDH
  (`ring::agreement`), `chacha20-poly1305@openssh.com` (`ring::aead::chacha20_poly1305_openssh`),
  `aes128-gcm@openssh.com` and `aes256-gcm@openssh.com` (`ring::aead`), SHA-256/512 and HMAC, and
  verification and signing for Ed25519, ECDSA P-256/P-384 and RSA with SHA-256/512.
- **Keys are parsed with the `ssh-key` crate**, parsing only, with its `encryption` feature for
  passphrase-protected OpenSSH keys (bcrypt-pbkdf). Every signature is made and checked by `ring`. The
  `rsa` crate is not added: it carries RUSTSEC-2023-0071, a timing side channel in its private-key
  operations. Stage 5 confirms `ssh-key` and its features pass `deny.toml` (MIT/Apache-2.0); if one
  does not, the key formats are parsed by hand in `nvs-ssh`.
- **What is offered, in order.** Key exchange: `curve25519-sha256`, `curve25519-sha256@libssh.org`,
  `ecdh-sha2-nistp256`, `ecdh-sha2-nistp384`. Host keys: `ssh-ed25519`, `ecdsa-sha2-nistp256`,
  `ecdsa-sha2-nistp384`, `rsa-sha2-512`, `rsa-sha2-256`. Ciphers: `chacha20-poly1305@openssh.com`,
  `aes256-gcm@openssh.com`, `aes128-gcm@openssh.com`. No MAC is negotiated, since every cipher is an
  AEAD. Compression: `none`.
- **Strict key exchange is always offered** (`kex-strict-c-v00@openssh.com`), which closes the Terrapin
  prefix-truncation attack. When the server does not offer it, ChaCha20-Poly1305 is not used, and an
  AES-GCM cipher is, which Terrapin does not affect.
- **Rekeying** happens after 1 GiB in either direction or one hour, as OpenSSH does, and whenever the
  server asks.
- **Sign-in** tries what the block holds, in the server's order: `publickey`, then `password`, then
  `keyboard-interactive` answering each prompt with the password (many servers offer only that). A
  banner from the server is read and discarded.
- **SFTP is protocol version 3** (draft-ietf-secsh-filexfer-02), the version OpenSSH and every hosted
  service speak, with the `posix-rename@openssh.com` and `fsync@openssh.com` extensions where offered.

**FTP is written by hand in `crates/nvs-stdlib/src/file_transfer/ftp.rs`**, as SMTP is in
`mail/smtp.rs`: a line protocol needs no crate.

- `FEAT` is read once after sign-in, then `OPTS UTF8 ON` when offered.
- `AUTH TLS` then `PBSZ 0` and `PROT P` for `explicit`, TLS from the first byte for `implicit`, both
  through `NvsTls`, always verified. The data connection is encrypted too, never `PROT C`.
- **Every data connection resumes the control connection's TLS session**, because vsftpd's
  `require_ssl_reuse` and FileZilla Server refuse one that does not. That needs a per-connection
  resumption store in the rustls `ClientConfig`; stage 4 adds it to `NvsTls` for this caller only.
- **Passive mode, `EPSV` first, then `PASV`.** The address in a `PASV` reply is ignored, and the data
  connection goes to the control connection's peer address with the reply's port. A server that names
  another address is how FTP bounce and server-side request forgery work, and curl does the same.
- Listings use `MLSD`/`MLST` where `FEAT` names them. Otherwise `LIST` is parsed in the Unix `ls -l`
  form and the DOS/IIS form, and a line in neither form is skipped and counted. `SIZE` and `MDTM` fill
  `metadata` without MLST.
- `REST` resumes `readStream` at an offset, and `APPE` is `append`.

**Paths are judged before they are joined** (`rule:security/writes-open-beneath-a-handle`'s order).
Each path is split on `/`, and a part that is `..`, empty, or holds NUL, CR or LF throws `RuntimeError`
before a byte is sent. An absolute path throws too: every path is relative to `root`. CR and LF matter
most on FTP, where a file name is part of a command line and `a\r\nDELE b` would send a second command.
The judged path is joined to `root` on the client. Symbolic links inside `root` on the server are the
server's own access control and are not resolved by the client.

**Bounds**, each a named constant with its reason in the code, and none of them a new `nvs.toml` key
(the user's call: no new limits):

- an FTP reply line at most 8 KiB, a multi-line reply at most 1 MiB, a listing line at most 64 KiB;
- an SSH packet at most 256 KiB (OpenSSH's own maximum), an SFTP message at most 256 KiB, and the
  version line and banner at most 64 KiB, as `Core\Imap`'s greeting cap is;
- a `walk` at most 64 folders deep, as `file-scan`'s walk is.

**Pipelining.** An SFTP read or write keeps up to 64 requests of 32 KiB in flight, as OpenSSH's `sftp`
does, so a transfer over a 50 ms link is not one round trip per 32 KiB. **What it spends:** up to 2 MiB
of buffers per transfer in progress, attributable to its request and charged to its memory ceiling, and
freed when the transfer ends. An FTP transfer holds one 64 KiB block. An SSH connection holds two packet
buffers of 256 KiB and the cipher state; an FTP connection holds its line buffer and, for FTPS, two
rustls connections. Stages 4 and 7 measure each and write the figures into the rule.

**Connections are pooled per block** (stage 9), because signing in to SFTP or FTPS costs several round
trips and a key exchange, and latency on the request path comes before simplicity. The pool is
`nvs_runtime::pool` under the key `file_transfer:<name>`, with `[mail]`'s defaults
(`crates/nvs-runtime/src/pool.rs:171`). **Request isolation comes first:** a connection goes back to the
pool only when it has no open SFTP handle, no unfinished FTP data connection and no unread reply.
Nothing a request does survives into the next one: FTP never sends `CWD` (every path is sent whole),
and `TYPE I` is sent before each transfer. A connection that fails any of these is closed, not
returned. A block's credentials are the pool's key, so a reload that changes them starts a new pool.
`pool = false` opens one connection per `open`.

**Tests.** An in-process FTP server in the tests (`crates/nvs-stdlib/src/file_transfer/ftp/tests.rs`),
on loopback with the test TLS config from `crate::mail::localhost`, as `Core\Imap`'s `Fake` server is
(`crates/nvs-stdlib/src/imap/session/tests.rs:27`). `nvs-ssh` has a test-only server role behind the
crate feature `test-server` (key exchange from the server's side, password, keyboard-interactive and
key sign-in, and an SFTP server over a temporary folder), so every SSH and SFTP test runs on every CI
runner, Windows and macOS included. Real interop is stage 8's: OpenSSH `sshd` and `vsftpd` started by
the test on Linux, unprivileged on a random port.

**Lanes.** Stages 1 to 3 run in order. Stage 4 (FTP, `nvs-stdlib`) and stages 5 and 6 (`nvs-ssh`)
touch disjoint files and can run side by side once stage 3 has landed. Stage 7 needs stages 3 and 6.
Stages 8 and 9 need 4 and 7, and touch disjoint files. Stage 10's proof files go to writing subagents.

**What stops it.** One attempt per item and two sessions per stage. A stage that does not close in two
sessions writes what holds it in the handoff, and the next session splits it. An interop failure that
is a real server's bug and not ours is recorded in the rule with the server and version, and worked
around only when the server is one stage 8 names.

## Stages

### 1. the rules say what Core\FileTransfer is, and the roster no longer declines FTP and SSH

- [ ] `docs/rules/core-classes/file-transfer.md:1`: the rule, added the way `docs/agent/doc-style.md`
  says, from this file's standing decisions: one API over a block, the block and what fails at boot,
  the grant, the members and `Entry`, paths judged before joined, the bounds, pipelining, the pool and
  its reset, and what each costs. Its Declined list is this file's.
- [ ] `docs/rules/core-classes/ssh-client.md:1`: the SSH client's rule: sans-I/O in `nvs-ssh`, `ring`
  for every primitive, `ssh-key` for parsing only, the algorithm lists in order, strict key exchange,
  rekeying, sign-in order, SFTP v3 and its two extensions, and the declined algorithms with reasons.
- [ ] `docs/rules/core-api/tier-roster.md:61`: `ftp` and `ssh2` leave the Declined list, which keeps
  `soap` alone, and `Core\FileTransfer` joins the Native placement at `:23`. The sentence claiming
  `Core\Storage` covers FTP is gone.
- [ ] `docs/rules/programs/framework-core-half.md:13` and `crates/nvs-stdlib/src/storage.rs:5`:
  unchanged in meaning (remote backends stay out of `Core\Storage`), and each points to
  `Core\FileTransfer` for a program that reaches a remote server.
- [ ] `bun nv names --check` over the new class and member names, and the result recorded here.
- [ ] `bun nv rules --render`, then `bun nv rules --check` is green.

### 2. nvs.toml reads [file_transfer.<name>] blocks and the file_transfer.connect grant

**Split:** none, one session.

- [ ] `crates/nvs-config/src/file_transfer.rs:1`: the block, modelled on `crates/nvs-config/src/imap.rs`,
  with every key in the table, the defaults and every refusal in *What fails at boot*. `host_key`
  takes a string or an array; `known_hosts_file` is parsed at boot with `ssh-key`'s parser, or by
  hand if stage 5 drops the crate, and a file with no line for the host fails at boot.
- [ ] `crates/nvs-config/src/lib.rs:87`: `pub mod file_transfer;` and the field on the config.
- [ ] `crates/nvs-config/src/capability.rs:201`: `FileTransferConnect`, `file_transfer.connect`, beside
  `ImapConnect`, in each of its three match tables.
- [ ] Reload: the blocks are reloadable as `[imap]`'s are, and the rule fragment that lists reloadable
  sections names them (`docs/rules/config/reloadability-is-its-own-field.md`).
- [ ] `crates/nvs-config/tests/file_transfer_config.rs:1`: each protocol boots, each refusal fails
  naming its key, and a secret file's content never appears in a message. The check `config` runs it.

### 3. Core\FileTransfer, its connection and its entries work over a directory block

**Split:** none, one session.

- [ ] `crates/nvs-stdlib/src/file_transfer/mod.rs:1`: `Core\FileTransfer::open`, the
  `Connection`, `Entry`, `Listing` and `Walk` classes, their registry rows and reference cards
  (`rule:core-api/reference-card`), the grant check, and the path judge. A protocol behind a trait
  with one implementation per backend, so stages 4 and 7 add a file each.
- [ ] `crates/nvs-stdlib/src/file_transfer/directory.rs:1`: every member over a local folder through
  the `fs.*` beneath-handle code `Core\IO` uses, as `crates/nvs-stdlib/src/imap/directory.rs` backs
  `Core\Imap`. `$atomic` writes and renames the part file the same way the remote backends will.
- [ ] `crates/nvs-stdlib/src/registry.rs`: the door table names `file_transfer.connect` for `open`.
- [ ] The connection is held by the request through `hold_open_connection`
  (`crates/nvs-runtime/src/ctx/held.rs:793`), so it closes when the request or run ends.
- [ ] `tests/conformance/core/file-transfer-a-directory-block-runs-every-member.nvst:1` and
  `tests/conformance/core/file-transfer-a-path-that-leaves-the-root-throws.nvst:1`: the checks
  `directory-round-trip` and `root` run them. A third case,
  `file-transfer-open-without-the-grant-throws.nvst`, opens a block the grant does not list.

### 4. every member works over FTPS and anonymous FTP

**Split:** none, one session. It can run beside stages 5 and 6.

- [ ] `crates/nvs-host/src/tls.rs:113`: a constructor for a client config with its own resumption
  store, used by one FTPS connection for its control and data connections. Every other caller keeps
  the shared config.
- [ ] `crates/nvs-stdlib/src/file_transfer/ftp.rs:1`: the control connection, `explicit`, `implicit`
  and `none`, sign-in, `FEAT`, `EPSV`/`PASV` with the reply's address ignored, `TYPE I`, and every
  member. `ftp/list.rs` parses MLSD facts, the Unix form and the DOS form.
- [ ] `crates/nvs-stdlib/src/file_transfer/ftp/tests.rs:1`: the in-process server, and a test per
  member over explicit, implicit and anonymous plain FTP, a `PASV` reply naming another address that
  is never connected to, a data connection that does not resume the session refused by the server
  side, a reply line past its bound, and a `LIST` fixture per form with odd names (spaces, a name
  starting with `-`, a non-UTF-8 byte). The check `ftps` runs them.
- [ ] The memory of one FTPS connection and one transfer is measured and written into the rule.

### 5. nvs-ssh speaks the SSH transport: key exchange, ciphers, host keys and rekeying

**Split:** none, one session. It can run beside stage 4.

- [ ] `crates/nvs-ssh/Cargo.toml:1` and `Cargo.toml`: the crate, its `ring` and `ssh-key` dependencies
  with the comment explaining the choice and the declined crates, and the `test-server` feature.
  `cargo deny check` (or the `bun nv` gate that runs it) is green with them.
- [ ] `crates/nvs-ssh/src/transport.rs:1`: the version exchange, the binary packet protocol with the
  packet bound, algorithm negotiation in the offered order, the four key exchanges, strict key
  exchange and the cipher choice it forces, the three ciphers, host-key signature checks for the five
  types, the fingerprint match against the pins or the `known_hosts` lines, and rekeying.
- [ ] `crates/nvs-ssh/src/server.rs:1` behind `test-server`: the server side of the same.
- [ ] `crates/nvs-ssh/tests/ssh_transport.rs:1`: every key exchange with every cipher, an unpinned
  host key refused before sign-in, a server without strict key exchange never given ChaCha20, a
  rekey after a test-only byte threshold, a packet past the bound refused. The check `ssh-transport`
  runs it.

### 6. nvs-ssh signs in with a password, keyboard-interactive or a key, and opens the SFTP subsystem

**Split:** none, one session.

- [ ] `crates/nvs-ssh/src/auth.rs:1`: `publickey` for Ed25519, ECDSA P-256/P-384 and RSA with
  SHA-256/512, OpenSSH and PKCS#8 key files, encrypted keys with their passphrase; `password`;
  `keyboard-interactive`; the order of *Standing decisions*.
- [ ] `crates/nvs-ssh/src/channel.rs:1`: one session channel, its window and flow control, and the
  `sftp` subsystem request. No other channel or request type is sent, and one the server opens is
  refused.
- [ ] The test server signs in each way, and `crates/nvs-ssh/tests/ssh_auth.rs:1` covers every key
  type, an encrypted key with the right and the wrong passphrase, a server that offers only
  keyboard-interactive, and a wrong password. The check `ssh-auth` runs it.

### 7. every member works over SFTP, with pipelined reads and writes

**Split:** none, one session.

- [ ] `crates/nvs-ssh/src/sftp.rs:1`: SFTP v3 messages, request ids, the message bound, and the two
  extensions; the test server's SFTP side over a temporary folder.
- [ ] `crates/nvs-stdlib/src/file_transfer/sftp.rs:1`: every member over `nvs-ssh` on `NvsTcp`, with
  reads and writes pipelined 64 deep, `move` with and without `posix-rename@openssh.com`, and the
  request deadline polled while a transfer runs.
- [ ] Rust tests beside it: each member against the test server, a 16 MiB `readStream` and `write`
  counted at no more than 16 MiB / (64 × 32 KiB) + 2 round-trip waits (a count, never a clock,
  `rule:programs/less-work-is-taken`), a reply with an unknown request id refused, and the memory of
  one connection and one transfer measured and written into the rule. The check `sftp` runs them.

### 8. interop tests run against OpenSSH and vsftpd on Linux, and fuzz targets cover every parser

**Split:** the fuzz targets go to one writing subagent while the session writes the interop tests.

- [ ] `crates/nvs-stdlib/tests/file_transfer_interop.rs:1`: starts `sshd` with a temporary config and
  host key, unprivileged on a random port, and `vsftpd` with `ssl_enable`, `require_ssl_reuse` and a
  test user, and runs every member against each. Its environment is declared the way
  [testing.md](../agent/testing.md) § *A test holds on every CI runner* says, so it runs where both
  binaries are installed and says why it did not run elsewhere. The check `interop` runs it in WSL.
- [ ] `.github/workflows/ci.yml`: the Linux job installs `openssh-server` and `vsftpd` and runs the
  interop test.
- [ ] `fuzz/fuzz_targets/ssh_packet.rs:1`, `sftp_message.rs:1`, `ftp_reply.rs:1` and
  `ftp_listing.rs:1`, with seeds under `fuzz/seeds/`, registered in `fuzz/Cargo.toml` as
  `reply_head.rs` is.

### 9. connections are pooled per block, and nvs prints a server's host key for the operator to pin

**Split:** none, one session.

- [ ] `crates/nvs-stdlib/src/file_transfer/pool.rs:1`: the pool under `file_transfer:<name>` with
  `[mail]`'s defaults, the checks before a connection is returned, and `pool = false`, modelled on
  `crates/nvs-stdlib/src/mail/pool.rs`.
- [ ] Tests beside it, modelled on `mail/pool.rs:168`: reuse, a connection with an open handle closed
  and not returned, a reload that changes credentials starting a new pool, and two requests that never
  see each other's state. The check `pool` runs them.
- [ ] `crates/nvs-cli/src`: a subcommand that connects to an SFTP block's host, prints each host key's
  type and `SHA256:` fingerprint and the `host_key = [...]` line to paste, and says to compare it with
  the provider's published fingerprint. It signs in to nothing. Its name follows the CLI's naming
  rules, which stage 1 records.
- [ ] Its help text and a CLI test.

### 10. the feature proofs of every new member

**Split:** one writing subagent per class (`FileTransfer`, `FileTransfer-Connection`,
`FileTransfer-Entry`), side by side, none of them builds. Each gets the members' signatures and cards
from stage 3, § *Text an end user reads* in AGENTS.md, and `Core\Imap`'s proofs as the model
(`bun nv proofs --id 'Core\Imap::open'`). Every example runs against a `directory` block with its
`nvs.toml`, as `docs/examples/core/Imap/` does. The session builds and runs `bun nv proofs --comments`
over every new `.nvs` once they all end.

- [ ] `Core\FileTransfer::open`: about.md, three examples (open a block, the same code over a
  directory in a test, a block the grant does not list), a bench, and
  `tests/hostile/core/FileTransfer/open/01-a-request-names-the-block.nvs`.
- [ ] Each `Core\FileTransfer\Connection` member: about.md, three examples, a bench with its growth by
  file size or entry count, and one attack. The attacks include a path from a request that climbs out
  of the root, a file name with CR and LF, a listing that never ends, and a file larger than the
  request's memory read with `readBytes`.
- [ ] Each `Core\FileTransfer\Entry` member: about.md, three examples, a bench and an attack, as
  `file-scan` writes `Core\IO\Entry`'s.
- [ ] `bun nv proofs --owed --goal file-transfer` names nothing, and the check `proofs` is green. Each
  feature it names is paid here, listed above or not.

### 11. the help, the reference and the website say how a program transfers files

- [ ] `website/src/content/docs/guides/how-to-use/transferring-files.mdx:1`: a guide page beside
  `receiving-mail.mdx`: an SFTP block with a pinned host key, an FTPS block, the grant, uploading a
  report atomically, downloading new files from a partner's folder, and the same code against a
  directory in a test. It follows § *Text an end user reads*.
- [ ] The configuration reference lists `[file_transfer.<name>]` and `[capabilities.file_transfer]`.
  An `Explore` subagent finds where `[imap.<name>]` is listed first.
- [ ] `website/src/data/changes.json`: `new` `Core\FileTransfer`, `new` `Core\FileTransfer\Connection`,
  `new` `Core\FileTransfer\Entry`, `new` the `[file_transfer]` block, and `new` the host-key command.
- [ ] Every handwritten website page that says Novis has no FTP or SFTP, or points to `Core\Process`
  for it, is reread and stamped (`bun nv site --stamp`). An `Explore` subagent lists them first.
- [ ] `bun nv site --check stale` and `changes` are green.

### 12. the goal is verified once, and every failure is fixed

- [ ] The newcomer review, [prose.md](../agent/prose.md) § *A newcomer reads it*, over every page,
  `about.md` and card this goal wrote, and `bun nv site --check review` is green.
- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

Written by the session that took the user's reversal of the `ftp` and `ssh2` decline. Nothing is
built yet. Every design choice is above, under *Standing decisions*. The one marked as the user's call
is not reopened.

### Next

Stage 1 from its first item. The checks name tests and cases that do not exist yet, and they fail
until stages 2 to 10 write them.

### Backlog
