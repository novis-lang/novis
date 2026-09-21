Checks that a signature belongs to a public key and covers a message. It returns nothing when the
check holds, and throws an error when it does not.

There is no `true` or `false` here, so there is no answer a program can forget to look at. A call
that returns means this message is the one that was signed, by whoever holds the private half of
this key. Put the call in a `try` block, and refuse the message in the `catch` block.

The algorithm is the kind of the key, exactly as it is for `Core\Crypto::sign`. Nothing that arrived
with the message chooses how the message is checked.

Every way a check can fail gives the same error: a changed message, a changed signature, a signature
of the wrong length, and a signature made under a different key. The error says nothing about which
of them happened, because that would tell a forger where to try next.

**Good to know:** an `X25519` key verifies nothing. That kind is for `Core\Crypto::agree`.
