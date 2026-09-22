Checks whether two digests or signatures are the same, and takes the same time wherever they
differ.

Use it every time you compare a secret value, such as a signature, a token or a digest, with the
value a visitor sent. A normal comparison stops at the first byte that differs. An attacker can
measure that time and guess a correct signature one byte at a time. `Core\Hash::equals` always
reads every byte, so the time shows nothing. Two values of different lengths return `false` at
once, because the length of a digest is never a secret.

**In plain words:** a normal comparison is like a guard who says "wrong" as soon as the first
letter of the password is wrong. This one always listens to the whole password first.

**Good to know:** both values must be `bytes`. This replaces PHP's `hash_equals`.
