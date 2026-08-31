---
summary: password hashing with no algorithm and no cost argument — the library picks the parameters, and `needsRehash` is how a stored hash learns it has fallen behind
keywords: password, hash, verify, needsRehash, argon2, argon2id, bcrypt, crypt, login, credential, rehash, salt, phc
---

`Core\Password::hash` takes a password and answers the string to store. It takes **nothing else**: no
algorithm, no memory cost, no time cost, no salt. PHP writes those at every call site, which means a
program's real cost is whichever call site was copied last and raising it is a grep. Here the parameters
belong to the library — Argon2id at OWASP's recommended floor — and `::needsRehash` is how a *stored*
hash finds out it has fallen behind. That is the whole reason a caller never names a cost.

The answer is a PHC string: the algorithm, the version, the cost parameters and the salt travel with the
digest. That is what lets `::verify` recompute under the settings a hash was *made* with, so credentials
written years ago still verify while new ones are written at today's cost.

`::hash` is one of exactly two operations that take a `secret` and answer something that is not one — the
other is `Core\Secret::reveal`. Storing a hash is the point, so the result is an ordinary `string` and
reaches the sinks a password never could.

Every call draws its own salt, so hashing one password twice answers two different strings and two users
who chose the same password are indistinguishable in the store. Compare with `::verify`, never with `==`.

**`::needsRehash` asks whether a hash is *weaker*, not whether it *differs*.** PHP compares the stored
options to the current ones for equality, so a hash written under stronger settings asks to be rehashed —
and a program doing as it was told silently downgrades it. Here a stronger stored hash is left alone.

A stored value that is not a hash this class wrote **throws** rather than answering `false`. A storage
layer handing back the wrong column otherwise looks exactly like every user typing the wrong password at
once, and one of those is worth waking someone up for.

```nvs
<?nvs
secret string $password = "correct horse battery staple";

// One argument, and the parameters come back out inside the answer.
string $stored = Core\Password::hash($password);
if (Core\Str::startsWith($stored, '$argon2id$v=19$')) {
    echo "stored under argon2id, v19\n";
}

// The bound, both sides of it.
secret string $offered = "correct horse battery staple";
secret string $wrong = "hunter2";
if (Core\Password::verify($offered, $stored)) {
    echo "the right password verifies\n";
}
if (!Core\Password::verify($wrong, $stored)) {
    echo "the wrong one does not\n";
}

// A hash made now is current by definition; one made under a lower cost is not.
string $old = '$argon2id$v=19$m=8192,t=1,p=1$jJ+hokoSJRsAzYsgfwhV6g$BGyWXoH11l0/GF4ezLqvtQgPY0T/4fv4DukErq9R0cI';
if (!Core\Password::needsRehash($stored) && Core\Password::needsRehash($old)) {
    echo "fresh is current, m=8192 has fallen behind\n";
}

// The usual shape at a login: verify, then upgrade what you just proved.
if (Core\Password::verify($offered, $stored) && Core\Password::needsRehash($stored)) {
    $stored = Core\Password::hash($offered);
}
echo "done\n";
```
```output
stored under argon2id, v19
the right password verifies
the wrong one does not
fresh is current, m=8192 has fallen behind
done
```
