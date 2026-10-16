Draws a fresh key for `Core\Crypto::seal` and `Core\Crypto::open`.

A key is 32 bytes of random data. There is no key size to choose, because both ciphers Novis offers
use one size. That is why this member takes no arguments. Every call gives a new key.

The type of the key is `secret bytes`. The `secret` part belongs to the type, so the program cannot
print the key, write it to a log, or put it in an error message. It can pass the key to `seal` and
`open`, and it can compare two keys with `==`.

To store a key or send it somewhere, you write one `Core\Secret::revealBytes` call. That call needs
a reason in text, so every place where a key leaves the program is easy to find.

**Good to know:** this is the member for a key nobody has to remember. For a key made from a
password that a person types, use `Core\Crypto::deriveKey`.
