---
looper: 1
title: "sending mail covers what a web app needs: attachments, inline images, styled HTML, names, threads and every common server"
summary: "Reached when Core\\Mail::send sends attachments and inline images, inlines a mail's CSS and builds its plain text from the HTML, writes a Message-ID and display names, takes only escaped HTML, reaches servers on port 465, with a private CA or with AUTH LOGIN and XOAUTH2, sends many messages over one connection, keeps captured mail for the debug viewer, and the rules, the feature proofs and the website say so."
context: [crates/nvs-stdlib/src/mail/mod.rs, crates/nvs-stdlib/src/mail/compose.rs, crates/nvs-stdlib/src/mail/smtp.rs, crates/nvs-config/src/tree.rs, crates/nvs-config/src/default.toml, docs/rules/programs/framework-core-half.md]
stages:
  - id: 1
    title: "a message carries a Message-ID, display names, thread headers and one-click unsubscribe, and an international domain is sent"
    status: done
    context: [docs/rules/errors/ambiguous-input-refused.md]
  - id: 2
    title: "a block reaches a server on port 465, with STARTTLS and no password, with its own CA, and with AUTH LOGIN, and an oversized message sends no DATA"
    status: done
    context: [crates/nvs-host/src/tls.rs, crates/nvs-stdlib/src/db/open.rs]
  - id: 3
    title: "a send reuses an open connection to its block, under the bounds [db.<name>.pool] has"
    status: open
    context: [crates/nvs-stdlib/src/db/registry.rs]
  - id: 4
    title: "the body is a string or Core\\Html\\Markup, the plain text is built from the HTML, and a message carries attachments and inline images"
    status: open
    context: [crates/nvs-stdlib/src/html.rs, crates/nvs-stdlib/src/mime.rs, crates/nvs-stdlib/src/registry.rs]
  - id: 5
    title: "a mail's CSS is inlined, Core\\Mail::preview returns what send would send, and a stylesheet link throws an error that names the fix"
    status: open
    context: [Cargo.toml, crates/nvs-stdlib/Cargo.toml]
  - id: 6
    title: "a block signs in with XOAUTH2, its token fetched by the host and reused until it expires"
    status: open
    context: [docs/rules/config/a-secret-is-a-file-whose-content-is-the-value.md]
  - id: 7
    title: "a block with a DKIM key signs every message it sends"
    status: open
  - id: 8
    title: "a capture block keeps its mail, Core\\Test\\SentMail reads every new part, and the debug viewer shows a mail page"
    status: open
    context: [crates/nvs-stdlib/src/test/records.rs, crates/nvs-runtime/src/test_records.rs, crates/nvs-runtime/src/spool.rs, viewer/src/router.ts, docs/rules/testing/a-test-reads-its-logs-and-mail.md]
  - id: 9
    title: "the rules, the reference, the examples, the feature proofs and the website say so"
    status: open
    context: [docs/examples/core/Mail/send/about.md, website/src/content/docs/guides/walkthroughs/after-response.mdx, website/src/data/changes.json, docs/agent/prose.md]
  - id: 10
    title: "the goal is verified once, and every failure is fixed"
    status: open
    gate: true
checks:
  - id: headers
    stage: 1
    name: "every message has a Message-ID on the from domain that send returns, a display name is encoded and never adds a header, thread headers take only message ids, unsubscribe writes both headers over https only, and an international domain is sent as punycode"
    run: [cargo, test, --lib, "mail::"]
    tests: [a_message_has_a_message_id_on_the_from_domain, send_returns_the_message_id, the_from_name_is_encoded_and_the_address_is_the_blocks, a_recipient_name_is_encoded_and_cannot_add_a_header, in_reply_to_and_references_take_only_message_ids, unsubscribe_writes_both_headers_and_takes_only_https, auto_submitted_is_written_unless_turned_off, an_international_domain_is_sent_as_punycode, a_non_ascii_local_part_needs_smtputf8]
    timeout: 1800
  - id: transport
    stage: 2
    name: "port 465 speaks TLS from the first byte, tls = starttls without a credential is required and verified, tls_ca_file replaces the anchors for one block, AUTH LOGIN is used where PLAIN is not offered, a message over the advertised SIZE sends no DATA, and the envelope sender is return_path with an optional bounce tag"
    run: [cargo, test, --lib, "mail::"]
    tests: [port_465_speaks_tls_from_the_first_byte, starttls_without_a_credential_is_required_and_verified, a_tls_ca_file_replaces_the_anchors_for_one_block, auth_login_is_used_when_plain_is_not_offered, a_message_over_the_advertised_size_sends_no_data, a_rejected_recipient_is_named_and_nothing_is_sent, the_return_path_is_the_envelope_sender, a_bounce_tag_goes_after_a_plus_in_the_return_path, a_bounce_tag_with_another_character_throws]
    timeout: 1800
  - id: pool
    stage: 3
    name: "a second send to a block reuses its connection, a connection the server closed is replaced once, an idle one is closed after its idle time, pool = false opens one per send, and a connection never serves a second block"
    run: [cargo, test, --lib, "mail::"]
    tests: [a_second_send_reuses_the_connection, a_connection_the_server_closed_is_replaced_once, an_idle_connection_is_closed_after_its_idle_time, pool_false_opens_one_connection_per_send, a_pooled_connection_never_serves_another_block]
    timeout: 1800
  - id: parts
    stage: 4
    name: "a Markup body sends HTML with a generated plain text, a string body sends plain text, attachments and inline images are nested mixed > related > alternative, every cid has its image and every inline image is used, and a non-ASCII file name is RFC 2231 encoded"
    run: [cargo, test, --lib, "mail::"]
    tests: [a_markup_body_sends_html_and_generated_text, a_string_body_sends_plain_text_only, the_text_option_replaces_the_generated_text, the_plain_text_keeps_links_lists_and_paragraphs, an_attachment_from_a_file_and_from_bytes_is_sent, an_attachment_type_is_detected_from_its_bytes, an_inline_image_src_is_its_content_id, a_cid_with_no_inline_image_throws, an_inline_image_that_is_not_used_throws, the_parts_nest_mixed_related_alternative, a_non_ascii_file_name_is_rfc_2231_encoded]
    timeout: 1800
  - id: tainted-html
    stage: 4
    name: "a tainted string is not an HTML body: it is sent as plain text, and Core\\Html\\Markup is the only way to send HTML"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/mail-a-tainted-string-is-sent-as-plain-text.nvst]
    timeout: 900
  - id: css
    stage: 5
    name: "style rules are inlined by the cascade, a style the author wrote wins, at-rules and pseudo-classes and rules that match nothing stay in <style>, a stylesheet link throws, and preview returns exactly what send puts in the message"
    run: [cargo, test, --lib, "mail::"]
    tests: [a_style_rule_is_copied_onto_every_element_it_matches, the_more_specific_rule_wins_and_a_later_one_breaks_a_tie, an_important_rule_beats_a_normal_one, a_style_attribute_the_author_wrote_wins, a_media_query_and_a_pseudo_class_stay_in_a_style_element, a_rule_that_matches_nothing_stays_in_a_style_element, an_inlined_rule_is_removed_from_its_style_element, a_stylesheet_link_throws_and_names_the_fix, preview_returns_what_send_puts_in_the_message, inlining_twice_gives_the_same_html]
    timeout: 1800
  - id: css-example
    stage: 5
    name: "a Novis program sees its <style> rules inlined in what Core\\Mail::preview returns"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/mail-preview-shows-the-css-inlined.nvst]
    timeout: 900
  - id: oauth
    stage: 6
    name: "an XOAUTH2 block fetches its token once and reuses it until it expires, takes the client-credentials and the refresh-token flow, needs an https token URL, and a refused token is an IOError that names the block"
    run: [cargo, test, --lib, "mail::"]
    tests: [a_client_credentials_token_is_fetched_once_and_reused, a_token_is_fetched_again_before_it_expires, a_refresh_token_block_sends_its_refresh_token, a_token_url_that_is_not_https_is_a_settings_error, a_refused_token_names_the_block_and_sends_nothing]
    timeout: 1800
  - id: dkim
    stage: 7
    name: "a DKIM block signs with relaxed/relaxed canonicalization, the RFC 8463 Ed25519 vector and an RSA signature verify, and the signed header list is fixed"
    run: [cargo, test, --lib, "mail::"]
    tests: [relaxed_canonicalization_matches_rfc_6376, the_rfc_8463_ed25519_vector_verifies, an_rsa_signature_verifies_with_its_public_key, the_signed_headers_are_the_fixed_list, a_key_shorter_than_2048_bits_is_a_settings_error]
    timeout: 1800
  - id: capture
    stage: 8
    name: "a capture block opens no connection and keeps every message, a kept message reaches the debug spool, and Core\\Test\\SentMail reads the message id, the names, the attachments and the HTML as sent"
    run: [cargo, test, --lib, "mail::"]
    tests: [a_capture_block_opens_no_connection, a_captured_message_reaches_the_spool, a_real_send_spools_only_its_event]
    timeout: 1800
  - id: sent-mail
    stage: 8
    name: "Core\\Test\\SentMail reads every part a message now has"
    run: [bun, nv, suite, tests/conformance/, --case, tests/conformance/core/test-sent-mail-reads-attachments-names-and-the-sent-html.nvst]
    timeout: 900
  - id: viewer
    stage: 8
    name: "the debug viewer lists captured mail and shows one in a sandboxed frame with its text, headers, attachments and CSS report"
    run: [bun, test, viewer/test/mail.browser.test.ts]
    timeout: 900
  - id: proofs
    stage: 9
    name: "every feature the goal's work reached has its feature proofs"
    run: [bun, nv, proofs, --gate, --goal, mail-complete]
    timeout: 1800
  - id: close
    stage: 10
    name: "the goal-end check is green: ci --check's checks of the tree's files, the owners and the goal's proofs"
    run: [bun, nv, close, --goal, mail-complete]
    timeout: 900
  - id: unknown-issuer-hint
    stage: 3
    name: "a certificate from an unknown issuer gives an IOError that names the block's tls_ca_file as the fix"
    run: [cargo, test, --lib, "mail::"]
    tests: [an_unknown_issuer_names_the_tls_ca_file]
---

# sending mail covers what a web app needs: attachments, inline images, styled HTML, names, threads and every common server

## Why

The user: mail is essential in every web app, and `Core\Mail` must have everything sending needs.
Today `Core\Mail::send` (`crates/nvs-stdlib/src/mail.rs:179`) sends a plain text and an optional
HTML string, and nothing else:

- **No attachments and no inline images.** The module doc (`mail.rs:74`) and
  `rule:programs/framework-core-half` say composition belongs to the program or a package. That is
  unbuildable: `send` takes no raw header and no raw body by design, so no package can compose an
  attachment into its arguments. Nobody can send one.
- **No `Message-ID`.** `compose` (`mail.rs:703`) writes Date, From, To, Cc, Subject and the MIME
  headers only. Spam filters score a missing id, some large receivers refuse it, and a relay does
  not add one for a remote client by default.
- **No display names.** `From: <noreply@x>` is the only form, and a named recipient is refused.
- **`html` takes a tainted string** (`Qual::Neutral`, `mail.rs:165`). A request value concatenated
  into it is HTML injection in a mail sent from the operator's domain. Every web response takes
  `Core\Html\Markup` for that reason (`rule:core-classes/html-auto-escape`).
- **Only STARTTLS with `AUTH PLAIN`.** No port 465, no TLS for a relay without a password, no private
  CA although `[db.<name>] tls_ca_file` exists (`crates/nvs-host/src/tls.rs:241`), no `AUTH LOGIN`,
  no XOAUTH2. Microsoft 365 and Google Workspace are retiring password sign-in for SMTP.
- **One TCP and TLS handshake per message**, so a batch of 1,000 pays 1,000 handshakes.
- **The hard parts are left to the developer:** CSS inlining for mail clients that drop `<style>`,
  a plain-text version of the HTML, `cid:` images, RFC 2231 file names, one-click unsubscribe.
- **A program cannot match a bounce or a reply to the mail it sent.** A user needs to mark each
  stored delivery bounced or answered: that takes the `Message-ID` back from `send`, a bounce address
  that names the delivery, and no out-of-office replies to a noreply sender. Reading the answers is
  the goal `mail-receive`, behind this one.

## Standing decisions

**`Core` composes the whole message.** `rule:programs/framework-core-half`'s row becomes "the
transport and the message": every part and every header a program can set is a typed parameter,
and `compose` stays the only writer of a `\r\n`. A raw MIME or raw header parameter for packages is
declined: it reopens the header injection the class exists to close.

**The call.** `Core\Mail::send(string $endpoint, array<string|Core\Mail\Address> $to, string
$subject, string|Core\Html\Markup $body, {options}): string`.

- `$body` a `string` sends `text/plain`. `$body` a `Core\Html\Markup` sends HTML with a plain-text
  alternative built from it. A tainted string is a string, so it is sent as text and can never be
  HTML. That is the conformance case `mail-a-tainted-string-is-sent-as-plain-text`.
- It returns the `Message-ID` it wrote, without angle brackets, so a program can store it, pass it
  to `inReplyTo` later, and match a bounce or a reply to it. A `messageId` option a program fills is
  declined: a random id cannot collide or be guessed, and the returned one serves every use.
- The options: `cc`, `bcc`, `replyTo` (each `string|Core\Mail\Address`), `text` (replaces the
  generated plain text; a `RuntimeError` with a string body), `fromName`, `attachments`
  (`array<Core\Mail\Attachment>`), `inReplyTo`, `references` (`array<string>`), `unsubscribe`,
  `autoSubmitted` (`bool`, default `true`), `bounceTag`.
- `Core\Mail\Address::of(string $address, string $name)` is a named address. A plain string stays an
  address only, never parsed for a name (`rule:errors/ambiguous-input-refused`).
- Pre-alpha: the old `text` positional and `html` option are removed, with no alias
  (BREAKING CHANGE in the commit; every example and conformance case is rewritten in its stage).

**Headers (stage 1).**
- `Message-ID: <128 random bits as hex@from-domain>`, from the CSPRNG the runtime already uses.
- `From: "<from_name>" <from>`. `[mail.<name>] from_name` is the default and `fromName` replaces
  the name only, never the address. A name is RFC 2047 encoded by the same path as `subject_line`
  (`mail.rs:570`), so it cannot add a header. A tainted `fromName` is allowed: the address, and so
  the domain a reader checks, stays the operator's.
- `inReplyTo` and `references` take message ids only (`local@domain`, no `<>`, no control byte,
  no space); anything else is a `RuntimeError`.
- `unsubscribe: string` takes an `https://` URL and writes `List-Unsubscribe: <url>` and
  `List-Unsubscribe-Post: List-Unsubscribe=One-Click` (RFC 8058). Another scheme is a
  `RuntimeError`. No `mailto:` form.
- `Auto-Submitted: auto-generated` (RFC 3834) is on every message unless `autoSubmitted: false`.
  Most mail a web app sends is automatic, and the header stops out-of-office replies from coming back
  to a noreply sender. `false` is for mail a person wrote, such as a helpdesk agent's answer. The
  user chose on by default.
- A non-ASCII domain is sent as punycode through the `idna` crate (already pulled in through
  `url` by stage 5's `css-inline`; stage 1 adds it directly). A non-ASCII local part is sent only
  when the server advertises `SMTPUTF8`, with `MAIL FROM:<…> SMTPUTF8`; otherwise an `IOError`
  naming the address, with nothing sent.
- `idna_adapter` is pinned to `~1.1` in the workspace `Cargo.toml`, its unicode-rs backend: six
  crates in the lock file in place of ICU4X's dozen, for the same UTS 46 answer. Stage 5's `url`
  resolves to the same adapter. A domain is converted only when it is not ASCII, with the STD3
  rules; an ASCII domain stays as written (`rule:errors/ambiguous-input-refused`'s unusual half).
- An address with a second `@` is refused: two readers split it at different `@`s.
- `Core\Mail\Address::of` checks the address when it is called and `send` checks it again, so a
  value rebuilt from bytes is checked too. It has no reader methods yet; stage 8's `SentMail` is
  where a test reads names back.

**Transport (stage 2).** New `[mail.<name>]` keys, each written into `default.toml` (`:864`) and
`MailEndpoint` (`crates/nvs-config/src/tree.rs:957`):
- `tls = "implicit" | "starttls" | "none"`. Default: `implicit` when `port = 465`, `starttls` when a
  credential is set, `none` otherwise. `none` with a credential is a settings error. `starttls`
  without a credential requires and verifies TLS, which is the internal-relay case `mail.rs:62`
  names.
- `tls_ca_file`, read by `NvsTls::over_bundle` (`tls.rs:241`) as `[db]` does, replacing the anchors
  for that block.
- `auth = "plain" | "login" | "xoauth2"`. Default: `PLAIN` when advertised, else `LOGIN`. Both only
  over TLS, as today.
- `SIZE`: when the server advertises a limit and the message is larger, an `IOError` names both
  sizes and no `DATA` is sent.
- `return_path` (default: `from`) is the envelope sender, `MAIL FROM`; the `From:` header stays
  `from`. The option `bounceTag: string` sends `MAIL FROM:<local+tag@domain>` of `return_path`, so a
  bounce names the delivery in its address even when a server strips the original headers. The tag
  is `A-Z a-z 0-9 . _ -`, 1 to 64 characters, and anything else is a `RuntimeError`. The program
  chooses only the tag, never the local part or the domain, so the envelope identity stays the
  operator's. The user chose this shape.
- A rejected `RCPT` stays all-or-nothing: the session sends `RSET`, and the `IOError` names every
  rejected address. Sending to part of a list is declined: a program that wanted that sends one
  message per recipient.
- The settings errors are boot errors (`E0601`) in a new `crates/nvs-config/src/mail.rs`, on
  `ldap.rs`'s shape, which also trust-checks `tls_ca_file` as `[ldap]` does. Besides `none` with a
  credential, it refuses an `auth` with no credential and a `tls_ca_file` beside `tls = "none"`:
  each would be read and never used. `endpoint_of` repeats the `tls` and `auth` checks as a
  `RuntimeError`, and `deliver` refuses to write a password on an unsecured session, so a tree that
  skipped the boot check still never sends one in the clear.
- `auth` takes `plain` and `login` until stage 6, which adds `xoauth2` to
  `nvs_config::mail::auth_from` and its refusal.
- `SIZE` is checked before `MAIL FROM`, on the message plus its five-byte terminator, so the server
  gets no transaction at all. `SIZE 0` and a bare `SIZE` mean no limit (RFC 1870). The `SIZE=`
  parameter on `MAIL FROM` is not sent: the check before it already covers the case.
- `251` (not local, will forward) is an accepted `RCPT`.

**The connection pool (stage 3).** The module doc's declined connection reuse (`mail.rs:81`) is
reversed: with TLS, each message pays a handshake, and one reused connection is countable less work
(`rule:programs/less-work-is-taken`). The shape is `[db.<name>.pool]`'s
(`crates/nvs-config/src/default.toml:853`): `pool = false`, or `[mail.<name>.pool]` with `max`
(default 4 per core), `idle` (default `30s`), `lifetime` (default `5m`). A reused connection sends
`RSET` first; a connection the server closed is replaced once, then the error stands. A connection
belongs to one block and is never handed to another. Memory: at most `max` sessions per block per
core, bounded by the configuration, never by traffic. The test proves reuse by a connection count,
never a clock.

**The body and its parts (stage 4).**
- `Core\Mail\Attachment::fromFile(path $path, {name, type})`, `::fromBytes(string $name, bytes
  $data, {type})`, `::inlineFile(path)`, `::inlineBytes(string $name, bytes $data, {type})`. A path
  is resolved and checked under `fs.read` like `Core\File`'s. `type` defaults to `Core\Mime`'s answer
  from the bytes, then `application/octet-stream`.
- An inline attachment has `src(): string`, the `cid:` URL to write in `<img src="…">`. Its
  Content-ID is random. Every `cid:` in the HTML needs an inline attachment of this message, and
  every inline attachment needs a `cid:` in the HTML; either miss is a `RuntimeError` that names it.
- The parts nest `multipart/mixed` > `multipart/related` > `multipart/alternative`, each level
  present only when needed. Every part stays base64, so the fixed boundaries stay safe; each level
  gets its own boundary name.
- A file name is RFC 2231 encoded when it is not ASCII; a control byte in it is a `RuntimeError`.
- The plain text built from HTML: paragraphs and headings become blank-line blocks, `<br>` a line
  break, list items `- ` lines, a link `text (url)`, an image its `alt`, a table cell separated by
  tabs, `<style>`, `<script>` and `<head>` dropped, entities decoded, lines wrapped at 78 columns.
  html5ever (already in `nvs-stdlib`) parses it.
- Memory: the message is composed in memory for the length of the send, about 1.37 times the
  attachments' size, under the request's ceiling. Streaming a file attachment into `DATA` is declined
  for now: memory is last in the ordering, and one path is simpler.

**CSS inlining (stage 5).** The user: it must be well thought out, and a developer must know it
exists and how to use it.

- **Always on, with no option.** Every HTML body is inlined inside `send` and `preview`. An option
  would be one more thing to forget. Inlining HTML that is already inlined changes nothing, because
  an author's `style` attribute wins (the test `inlining_twice_gives_the_same_html`). An opt-out is
  declined until a real case needs one.
- **The engine is `css-inline` 0.25 with `default-features = false`.** It is built on html5ever
  `^0.40.1`, the version this tree already has, plus Servo's `cssparser` and `selectors`, the parser
  and matcher Firefox uses. With its `http`, `file`, `cli` and cache features off it reads nothing
  from the network or the disk. A matcher of our own is declined: specificity mistakes would be
  silent. Fallback, if 0.25 cannot keep the rules below in `<style>`: split the stylesheet with
  `cssparser` ourselves before handing over the inlinable part.
- **What happens to each rule.** A rule that matches elements is copied onto each of them by the
  cascade: specificity, then source order, `!important` above normal, and an author's `style`
  attribute above a normal rule. Then it is removed from `<style>`. These stay in one `<style>` in
  `<head>`: at-rules (`@media`, `@font-face`, `@supports`, `@keyframes`), pseudo-classes and
  pseudo-elements (`:hover`, `::before`), and **every rule that matched nothing**. A rule aimed at an
  attribute that a mail client adds later (Outlook's `[data-ogsc]`, Gmail's `u + .body`) matches
  nothing at send time and must survive.
- **`<link rel="stylesheet">` throws a `RuntimeError`** that says what to do: "A mail cannot load a
  stylesheet. Put the CSS in a `<style>` element. `Core\Mail` copies each rule into the elements it
  matches." Nothing is fetched or read. This error is where a developer who tries the obvious thing
  learns that inlining exists.
- **`Core\Mail::preview(string $subject, string|Core\Html\Markup $body, {text, attachments}):
  Core\Mail\Preview`** builds the body exactly as `send` would, sends nothing and needs no endpoint.
  `Core\Mail\Preview` has `html(): Core\Html\Markup` (after inlining), `text(): string`,
  `keptCss(): array<string>` (the rules left in `<style>`) and `unusedCss(): array<string>` (the
  kept rules that matched nothing). It is what an example prints to show the inlined result, what a
  "view this mail in your browser" page returns, and what a test checks without sending.
- **Where a developer learns it.** The `send` and `preview` reference cards say it in the first
  sentence of `$body`. The `about.md` of `send` has a section with an example whose `.out` shows the
  inlined HTML. The website's new how-to page has a "Styling an HTML mail" section with the same
  before and after. The debug viewer's mail page (stage 8) shows the HTML as written and as sent, the
  kept rules and the unused ones. `Core\Test\SentMail::html()` returns the inlined HTML.
- **Size.** Gmail cuts a message over 102 KB. The viewer's mail page shows a note above that size.
  Nothing throws.
- **Cost.** One `css-inline` parse plus the one html5ever parse of stage 4 per HTML message. Two
  parses are taken over our own matcher for correctness; the bench of stage 9 records the cost, and
  a single shared tree is the backlog item if the bench shows it matters. Memory: two trees per
  send, freed when it ends. Dependency trade-off: `css-inline`, `cssparser`, `selectors`, `url`,
  `idna` and their small helpers, with no network or async code.

**XOAUTH2 (stage 6).** The keys `oauth_token_url` (must be `https://`), `oauth_client_id`,
`oauth_client_secret` or `oauth_client_secret_file`, `oauth_scope`, and optional
`oauth_refresh_token_file`. No refresh token: the client-credentials grant (Microsoft 365). With one:
the refresh-token grant (Google). The host fetches the token over its own HTTP client, with the
compiled-in anchors, not through `Core\Http`: the URL comes from root-owned configuration, the same
reason `resolve` (`mail.rs:429`) does not pin. The token is cached per block per process until 60
seconds before `expires_in`. Memory: one token per configured block, bounded by the configuration.
A token request that fails is an `IOError` naming the block and the token server's status, with
nothing sent.

**DKIM (stage 7).** The keys `dkim_selector`, `dkim_key_file` (PKCS#8 PEM, RSA of 2048 bits or
more, or Ed25519) and `dkim_domain` (default: the domain of `from`). The signature is written over
the composed message with relaxed/relaxed canonicalization, over the fixed list From, To, Cc,
Subject, Date, Message-ID, MIME-Version, Content-Type, Reply-To, In-Reply-To, References,
List-Unsubscribe and List-Unsubscribe-Post (those present). Signing uses `ring` 0.17, already in the
tree. The signing identity is the deployment's, like `from`, so there is no call-site parameter.

**Capture and the viewer (stage 8).** `[mail.<name>] capture = true` makes a block keep its mail and
open no connection, as `nvs test` does (`crates/nvs-runtime/src/test_records.rs:81`). A capture
block needs `from` and no `host`. A kept message is written to the debug spool when recording is on,
so the viewer can show it. A real send spools only its event (`spool.rs:2173`), never the content:
a password-reset link must not land on disk. The viewer gets a Mail page (route in
`viewer/src/router.ts`, view `viewer/src/views/mail.tsx`): a list, then one message with its HTML
in an `<iframe sandbox>` (no scripts, no remote loads), its text, headers, attachments as downloads
and the CSS report. `Core\Test\SentMail` gains `messageId()`, `fromName()`, `attachments()`
(`array<Core\Mail\Attachment>`, with `name()`, `type()`, `bytes()` and `isInline()`), `inReplyTo()`,
`references()`, `unsubscribe()`, `autoSubmitted()` and `bounceTag()`; `html()` and `text()` return
what was sent.

**Background sending is not a new member.** `Core\Task::runAfterResponse` already sends after the
page is out (`website/src/content/docs/guides/walkthroughs/after-response.mdx`), and `Core\Queue`
retries. The about.md and the how-to page show both.

**Out of scope, declined:** S/MIME and PGP, a `mailto:` unsubscribe, partial sends, a free-form
header map. Receiving mail is the goal `mail-receive`, behind this one.

**Rules.** `rule:programs/framework-core-half` changes its `Core\Mail` row. A new fragment
`docs/rules/core-classes/mail-message.md` holds the call, the headers, the parts, CSS inlining, the
plain text, preview and § *Declined*. A new fragment `docs/rules/core-classes/mail-transport.md` holds
TLS, auth, the pool, XOAUTH2, DKIM and capture. The module doc of `mail.rs` keeps the reasoning and
points at both; its `# Attachments …` section and the declined reuse are rewritten, never kept as
history. `rule:testing/a-test-reads-its-logs-and-mail` names the new readers.

**Module layout.** `mail.rs` is split into `crates/nvs-stdlib/src/mail/` in stage 1, before it
grows: `mod.rs` (the class rows, `send`), `compose.rs`, `smtp.rs` (the session and `deliver`), and
later `pool.rs`, `parts.rs`, `css.rs`, `oauth.rs`, `dkim.rs`. One `#[cfg(test)]` module per file.

**What stops it.** One attempt per item and two sessions per stage. An item that cannot land in one
attempt is written to `### Backlog` with what blocked it.

**Lanes.** It runs on the chain after `deeper-debug-records`. Stage 8 needs that goal's mail event
and run list: its gate is that `docs/goals/deeper-debug-records.md` no longer exists. Stages 1 to 7
touch only `crates/nvs-stdlib/src/mail/`, `crates/nvs-config/` and `Cargo.toml`, and run in order
because they share `compose.rs` and `smtp.rs`.

## Stages

### 1. a message carries a Message-ID, display names, thread headers and one-click unsubscribe, and an international domain is sent

- [x] `crates/nvs-stdlib/src/mail.rs:1`: split into `crates/nvs-stdlib/src/mail/{mod,compose,smtp}.rs`
  per § *Module layout*; `crates/nvs-stdlib/src/lib.rs:182` keeps `mod mail;`. No behaviour change.
- [x] `crates/nvs-stdlib/src/mail.rs:703` (`compose`): `Message-ID`, and `From` with `from_name`;
  `send` returns the id (`mail.rs:179`, `return_ty`). `rule:programs/framework-core-half`.
- [x] `crates/nvs-config/src/tree.rs:957` and `crates/nvs-config/src/default.toml:864`: `from_name`.
- [x] `crates/nvs-stdlib/src/mail/mod.rs:162` (`OPTIONS`): `fromName`, `inReplyTo`, `references`,
  `unsubscribe`, `autoSubmitted`; `Core\Mail\Address` as a new class row; `cc`, `bcc`, `replyTo`
  and `$to` take `string|Core\Mail\Address`.
  - [x] the five options, read in `send` and written by `compose` (`compose::Headers`)
  - [x] `Core\Mail\Address::of(string $address, string $name)` as a class row
  - [x] `$to`, `cc`, `bcc` and `replyTo` take `string|Core\Mail\Address`; `compose::address_line`
    writes each name through `compose::mailbox`
- [x] `crates/nvs-stdlib/src/mail/compose.rs:44` (`address_of`): punycode for a non-ASCII domain; a
  non-ASCII local part is kept for `SMTPUTF8`, checked by `smtp::envelope_sender`.
- [x] The nine tests of the check `headers`, then `cargo test --lib mail::` once.
  - [x] five in `compose.rs`: `a_message_has_a_message_id_on_the_from_domain`,
    `the_from_name_is_encoded_and_the_address_is_the_blocks`,
    `in_reply_to_and_references_take_only_message_ids`,
    `unsubscribe_writes_both_headers_and_takes_only_https`,
    `auto_submitted_is_written_unless_turned_off`
  - [x] `send_returns_the_message_id`, `a_recipient_name_is_encoded_and_cannot_add_a_header`,
    `an_international_domain_is_sent_as_punycode`, `a_non_ascii_local_part_needs_smtputf8`
  - [x] three conformance cases for `Core\Mail\Address::of` (`tests/conformance/core/mail-*`)

### 2. a block reaches a server on port 465, with STARTTLS and no password, with its own CA, and with AUTH LOGIN, and an oversized message sends no DATA

- [x] `crates/nvs-config/src/tree.rs:957` and `default.toml:864`: `tls`, `tls_ca_file`, `auth` and
  `return_path`, each with its default and settings errors per § *Transport*; the `bounceTag` option
  in `OPTIONS` (`crates/nvs-stdlib/src/mail/mod.rs:162`), read by `endpoint_of` (`mod.rs:511`).
  - [x] the four `MailEndpoint` fields and their `[mail.default]` lines
  - [x] `crates/nvs-config/src/mail.rs`: the defaults, the boot refusals, `tls_ca_file` trust-checked
  - [x] `Endpoint` and `endpoint_of` read them, and `bounceTag` is slot 13
- [x] `crates/nvs-stdlib/src/mail/smtp.rs:91` (`Session::open`) and `:121` (`secure`): implicit
  TLS, and `over_bundle` (`crates/nvs-host/src/tls.rs:241`) for `tls_ca_file`.
- [x] `crates/nvs-stdlib/src/mail/smtp.rs:360` (`deliver`): `AUTH LOGIN`, `SIZE`, `RSET` with every
  rejected address named. `MAIL FROM` from `return_path` and the bounce tag goes through
  `envelope_sender` (`smtp.rs:326`), which already adds `SMTPUTF8`.
  - [x] `authenticate` picks `PLAIN` or `LOGIN`
  - [x] `size_limit` stops the send before `MAIL FROM`
  - [x] every `RCPT` reply is read, and a rejection sends `RSET` and names each address
  - [x] `bounce_sender` writes `local+tag@domain`
- [x] The nine tests of the check `transport` over a fake server on a `std::net::TcpListener`, as
  `a_reply_is_capped_per_line_and_per_reply` (`smtp.rs:495`) has, then `cargo test --lib mail::`
  once.

### 3. a send reuses an open connection to its block, under the bounds [db.<name>.pool] has

- [ ] `crates/nvs-config/src/tree.rs:968` (`MailEndpoint`), `crates/nvs-config/src/mail.rs`
  (`validate`) and `default.toml:868`: `pool` and `[mail.<name>.pool]`, shaped like
  `[db.<name>.pool]` (`default.toml:853`) and read as `crates/nvs-config/src/ldap.rs` `pool_for` does.
- [ ] `crates/nvs-stdlib/src/mail/pool.rs:1` (new): per block, per core; `RSET` on reuse, one
  replacement, idle and lifetime closing. `deliver` (`smtp.rs:552`) takes a pooled session.
- [ ] The five tests of the check `pool`, counting connections at the fake server, then
  `cargo test --lib mail::` once.
- [ ] `handshake` (`crates/nvs-stdlib/src/mail/smtp.rs`): a certificate from an unknown issuer gives
  an `IOError` that says the server's CA is not trusted and names `[mail.<name>] tls_ca_file` as the
  fix. Every other handshake failure keeps its message. A user met the bare `UnknownIssuer` on a
  server with a company CA and had no way to find the setting. Check `unknown-issuer-hint`.

### 4. the body is a string or Core\Html\Markup, the plain text is built from the HTML, and a message carries attachments and inline images

- [ ] `crates/nvs-stdlib/src/mail.rs:179` (`CLASS`): `$body` as `CoreTy::Union` of text and
  `Instance(Core\Html\Markup)`; the `text` option; `html` removed. `rule:core-classes/html-auto-escape`.
- [ ] `crates/nvs-stdlib/src/mail/parts.rs:1` (new): `Core\Mail\Attachment` and its four
  constructors, `src()`, `Core\Mime` detection (`crates/nvs-stdlib/src/mime.rs:1`), RFC 2231 names,
  the cid checks.
- [ ] `crates/nvs-stdlib/src/mail.rs:703` (`compose`): the mixed > related > alternative nesting.
- [ ] `crates/nvs-stdlib/src/mail/parts.rs:1`: the plain text from HTML over html5ever, per § *The
  body and its parts*.
- [ ] Every caller of the old shape: `docs/examples/core/Mail/send/01-sending-an-order-receipt.nvs:1`,
  `03-a-contact-form.nvs:1`, the `mail-*` and `test-sent-mail-*` cases under `tests/conformance/core/`,
  and what `rg "Core\\\\Mail::send"` lists outside `crates/`.
- [ ] `tests/conformance/core/mail-a-tainted-string-is-sent-as-plain-text.nvst:1` (new).
- [ ] The eleven tests of the check `parts`, then `cargo test --lib mail::` once.

### 5. a mail's CSS is inlined, Core\Mail::preview returns what send would send, and a stylesheet link throws an error that names the fix

- [ ] `Cargo.toml:1` and `crates/nvs-stdlib/Cargo.toml:1`: `css-inline = { version = "0.25",
  default-features = false }`; `cargo tree -i html5ever` shows one version.
- [ ] `crates/nvs-stdlib/src/mail/css.rs:1` (new): the inliner with the rules of § *CSS inlining*,
  the `<link>` error, and the kept and unused lists.
- [ ] `crates/nvs-stdlib/src/mail.rs:179`: `preview` and `Core\Mail\Preview` with its four readers;
  `send` and `preview` share one function, so they cannot differ.
- [ ] `crates/nvs-stdlib/src/mail.rs:211` (`SEND_DOC`): `$body`'s first sentence says the CSS is
  inlined; `preview`'s card says what it is for.
- [ ] `tests/conformance/core/mail-preview-shows-the-css-inlined.nvst:1` (new).
- [ ] The ten tests of the check `css`, then `cargo test --lib mail::` once.

### 6. a block signs in with XOAUTH2, its token fetched by the host and reused until it expires

- [ ] `crates/nvs-config/src/tree.rs:957` and `default.toml:864`: the `oauth_*` keys and their
  settings errors.
- [ ] `crates/nvs-stdlib/src/mail/oauth.rs:1` (new): both grants, the per-block cache, `AUTH
  XOAUTH2` in `authenticate` (`smtp.rs:447`), and `xoauth2` in `nvs_config::mail::auth_from`.
- [ ] The five tests of the check `oauth`, against a fake token server and the fake SMTP server,
  then `cargo test --lib mail::` once.

### 7. a block with a DKIM key signs every message it sends

- [ ] `crates/nvs-config/src/tree.rs:957` and `default.toml:864`: `dkim_selector`, `dkim_key_file`,
  `dkim_domain`; a short RSA key is a settings error.
- [ ] `crates/nvs-stdlib/src/mail/dkim.rs:1` (new): canonicalization, the header list, RSA and
  Ed25519 over `ring`; called on the composed message before `DATA`.
- [ ] The five tests of the check `dkim`, then `cargo test --lib mail::` once.

### 8. a capture block keeps its mail, Core\Test\SentMail reads every new part, and the debug viewer shows a mail page

Gate: `docs/goals/deeper-debug-records.md` does not exist.

**Split:** the viewer page and its browser test go to one writing subagent once the spool carries a
kept message; its files (`viewer/`) are disjoint from the Rust items.

- [ ] `crates/nvs-config/src/tree.rs:957`: `capture`; `crates/nvs-stdlib/src/mail.rs:1203` (`send`)
  keeps the message for a capture block as `nvs test` does.
- [ ] `crates/nvs-runtime/src/spool.rs:2173`: a kept message is spooled with its parts and its CSS
  report; a real send stays an event.
- [ ] `crates/nvs-stdlib/src/test/records.rs:438`: the new `SentMail` readers;
  `tests/conformance/core/test-sent-mail-reads-attachments-names-and-the-sent-html.nvst:1` (new).
- [ ] `viewer/src/views/mail.tsx:1` (new) and `viewer/src/router.ts:1`: the Mail page per § *Capture
  and the viewer*; `viewer/test/mail.browser.test.ts:1` (new).
- [ ] The three tests of the check `capture`, then `cargo test --lib mail::` and
  `bun test viewer/test/mail.browser.test.ts` once.

### 9. the rules, the reference, the examples, the feature proofs and the website say so

**Split:** the examples, the attack and the bench go to one writing subagent once the rule fragments
are written, and the how-to page to a second; their files are disjoint. Each brief names AGENTS.md
§ *Text an end user reads*.

- [ ] `docs/rules/programs/framework-core-half.md:12`, `docs/rules/core-classes/mail-message.md:1`
  (new), `docs/rules/core-classes/mail-transport.md:1` (new) and
  `docs/rules/testing/a-test-reads-its-logs-and-mail.md:7`, then `bun nv rules`.
- [ ] The feature proofs `bun nv proofs --owed --goal mail-complete` names. The examples it will
  need include: an invoice with a PDF attachment, a mail with a logo as an inline image, a styled
  mail with its inlined result printed by `Core\Mail::preview`, a reply in a thread through
  `inReplyTo`, a newsletter with `unsubscribe`, and sending after the response with
  `Core\Task::runAfterResponse`. Benches: inlining a typical styled mail, and composing a mail with
  a 1 MB attachment. Attacks: a header through a display name, a file name and a message id; a
  tainted string sent as a body; a `cid:` pointing at another message; a stylesheet link.
- [ ] `docs/examples/core/Mail/send/about.md:1`: rewritten, with a section on styling a mail.
- [ ] `website/src/content/docs/guides/how-to-use/sending-mail.mdx:1` (new): the settings block,
  sending, names, attachments and inline images, "Styling an HTML mail" with the before and after,
  threads, unsubscribe, sending in the background, testing and the viewer.
- [ ] `docs/reference/tools/20-config.md:1`: every new `[mail.<name>]` key.
- [ ] `website/src/data/changes.json:1`: entries `changed` `Core\Mail::send`, `new`
  `Core\Mail::preview`, `new` `Core\Mail\Attachment`, `new` `Core\Mail\Address`, `changed`
  `Core\Test\SentMail`, `changed` `[mail]` settings. Then `bun nv site --check stale`, `changes` and
  `review` green.
- [ ] `bun nv proofs --owed --goal mail-complete` names nothing.

### 10. the goal is verified once, and every failure is fixed

- [ ] Run the check the pack names for this stage: `looper verify` at a full gate, or a build and the tests this goal wrote, by name, at a close.
- [ ] Only if it is red: fix every failure it names in one pass, never deleting or loosening a test, then run it once more, and it is green.

## Handoff

### State

The user chose `Core` composing the message, CSS inlining, `Auto-Submitted` on by default and
`return_path` with `bounceTag`; every other choice under § *Standing decisions* is a session's.

Stages 1 and 2 are done. Stage 1: `send` returns the `Message-ID` it wrote, takes `fromName`,
`inReplyTo`, `references`, `unsubscribe`, `autoSubmitted` and `Core\Mail\Address::of` recipients,
and sends a non-ASCII domain as punycode; the nine `headers` tests pass. The nightly CI failures
the sweep named (run 45609291264) ran on `53e94a9181`, which `1c3196b0e8` fixed.

Stage 2: `[mail.<name>]` has `tls`, `tls_ca_file`, `auth` and `return_path`, judged at boot by
the new `crates/nvs-config/src/mail.rs` (`validate`, `canonicalize`, `tls_from`, `auth_from`), and
`send` has `bounceTag` (slot 13, `args: [14]`). `smtp.rs` opens implicit TLS in `Session::open`,
verifies with `tls_ca_file` through `handshake`, sends `AUTH LOGIN` in `authenticate`, checks
`SIZE` in `size_limit`, reads every `RCPT` reply and sends `RSET` on a rejection, and writes
`local+tag@domain` in `bounce_sender`. `deliver` takes an `Envelope` (sender and recipients). The
nine `transport` tests pass (`looper check --name transport`), over the fake server `serve` in
`smtp.rs`'s tests, which speaks plain, implicit TLS or `STARTTLS` with an `rcgen` certificate.
`a_mail_block_s_tls_defaults_and_refusals` in `nvs-config` passes too.

### Next

Stage 3, the pool. Start with the keys in `MailEndpoint` (`crates/nvs-config/src/tree.rs:968`)
and `crates/nvs-config/src/mail.rs` `validate`, read through `crate::db::pool_of` as
`crates/nvs-config/src/ldap.rs` `pool_for` does. Then `crates/nvs-stdlib/src/mail/pool.rs`: the
`ldap` pool in `crates/nvs-stdlib/src/ldap/mod.rs` is the nearest per-block, per-core pool to copy.
`deliver` (`smtp.rs:552`) splits into opening a `Session` (greeting, TLS, `authenticate`) and one
transaction (`SIZE`, envelope, `DATA`), so a pooled session runs only the second half after `RSET`.
The fake server `serve` (`smtp.rs` tests) accepts one connection; the pool tests need it to accept
several and count them.

Expected red until stage 9: `bun nv site --check stale` and `proofs` for `Core\Mail::send` and
`Core\Mail\Address::of` (no `about.md`, examples, bench, attack or website page yet), and the
reference-card review for `send`, whose `bounceTag`, `RuntimeError` and `IOError` texts changed.
No unit test or conformance case is expected red.

### Backlog
