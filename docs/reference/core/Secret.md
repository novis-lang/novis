---
summary: the one narrow way a value loses the `secret` qualifier — a call that says so by name and carries a written reason
keywords: secret, reveal, revealBytes, credential, password, token, api key, redaction, escape hatch, unwrap, expose
---

`Core\Secret::reveal` answers its operand with the `secret` qualifier dropped, and `::revealBytes` does the
same one base over. A `secret` value is refused at every sink that would disclose it — `echo` and `print`,
an interpolation, `Core\Json::encode`, `Core\Serialize::encode`, a `spawn script` boundary, a `Throwable`
message, a debug dump — and each of those refusals tells you to call this. That is the whole design: there
is no generic `unwrap()`, because a catch-all invites false confidence. Disclosure is a line you write, at
the one call site where handing the secret over is the point, and `grep` finds every one of them.

The second argument is that reason, written for the next reader. Nothing consumes it at run time.

Revealing removes `secret` and nothing else. A value that was also `tainted` still is: where the value came
from is a different question from who may see it, so `Core\Secret::reveal` over a `secret tainted string`
answers a `tainted string` and the sinks that refuse untrusted input still refuse it. Revealing a value that
was never `secret` is the identity and is legal — refusing it would cost a diagnostic and prevent no
exposure.

```nvs
<?nvs
secret string $token = "hunter2";
secret bytes $key = "raw-key" as bytes;

// Without these two lines, every `echo` below is a compile error naming
// ADR 0033 § 4's terminal sink.
string $revealed = Core\Secret::reveal($token, "this command exists to print the token");
bytes $raw = Core\Secret::revealBytes($key, "the signer takes the key as bytes");

echo $revealed, "\n";
echo "Bearer {$revealed}\n";
echo Core\Bytes::length($raw), "\n";

// Nothing is normalized on the way through: the value that comes back is the
// value that went in.
secret string $padded = "  spaced  ";
echo "[", Core\Secret::reveal($padded, "the padding is part of the credential"), "]\n";
```
```output
hunter2
Bearer hunter2
7
[  spaced  ]
```
