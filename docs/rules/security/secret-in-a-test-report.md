An assertion diff is simultaneously output, a log line, a dump and a `Throwable` message — all four of
the things a `secret` value may not become (`rule:security/secret-sinks-refuse`). The assertion still
runs; the **report is redacted, always**, with no flag and no build mode that lifts it, and the
failure object's own diff is redacted too, so catching the failure does not recover the value.

Byte length is reported, because the length is not the secret. Debugging a redacted failure is
genuinely harder, and the intended answer is to assert on a derived value — a digest of the token
rather than the token.

Separately and always, the reporter escapes control characters in every value it renders. A `tainted`
string is not confidential and may be shown, but a fixture full of terminal escapes must not be able
to rewrite the developer's terminal from inside a failure message.
