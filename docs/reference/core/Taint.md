---
summary: the one way a value loses the `tainted` qualifier when no sink-named launderer fits — a call that says so by name and carries a written reason
keywords: taint, tainted, assertTrusted, trust, untrusted input, injection, sanitize, allowlist, escape hatch, validation
---

`Core\Taint::assertTrusted` answers its operand with the `tainted` qualifier dropped, on your own written
authority. Every other way out of `tainted` is *sink-named*: `Core\Html::escape` for HTML text,
`Core\Regex::quote` for a pattern, `Core\Uri::encodeComponent` for a URI component. That is deliberate — a
value safe for HTML text is not safe for a shell argument, and a generic `sanitize()` would invite exactly
the false confidence taint tracking exists to prevent. This member is the one case that rule cannot cover:
you validated the value yourself, and you need to say so.

It is modeled on the `unsafe` keyword's job in Rust rather than on a cast. It is forbidden by default, it is
rare, it is greppable by its own name, and it carries in the source the reason the value can be trusted.
Nothing reads that reason at run time; it is written for the next person to read the line.

The place it is not a fallback but the only route is a database host. `Core\Db\Settings.host` refuses a
`tainted` value and has **no launderer of its own**, because no string check can establish that a hostname
is safe to send credentials to — a malicious server can answer any query with a `LOCAL INFILE` request and
read files off the application host. An Adminer-style tool where a human genuinely types the host is the
case that call site exists for, and this is its honest spelling.

Asserting a value that was never `tainted` is the identity and is legal: refusing it would cost a
diagnostic and prevent no injection.

The two qualifiers are independent bits, and this member answers on one of them. A `secret` operand is
refused outright — `Core\Secret::reveal` is the escape hatch on that axis. A value carrying both reveals
first, whose answer is still `tainted`, and asserts second; the other order does not compile.
