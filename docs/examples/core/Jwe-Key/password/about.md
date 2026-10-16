Makes a `Core\Jwe\Key` from a password. Use it when a person chooses the secret, for example a
password that protects an exported file. Any text is a password, and no length is too short or too
long.

A password is not a key, so `Core\Jwe::encrypt` first turns it into a key with a slow key
derivation (PBKDF2). The slow step makes it expensive for an attacker to guess many passwords.
A token made with this key uses the JWE algorithm `PBES2-HS256+A128KW`.

Only the exact same text opens the token. A password that differs in one letter, or only in upper
and lower case, throws a `RuntimeError` in `Core\Jwe::decrypt`.

Every call runs the slow step again. For a secret that a program makes itself, `Core\Jwe\Key::shared`
is much faster.

**The examples below** encrypt with a password, show that the case of each letter counts, and
protect an exported file.
