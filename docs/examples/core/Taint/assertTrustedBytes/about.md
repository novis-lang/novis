`Core\Taint::assertTrustedBytes()` returns `tainted bytes` as plain `bytes`, after your own check.

Bytes that may contain data from outside your program have the type `tainted bytes`.
`Core\Serialize::decode` does not accept them. `Core\Serialize::encode` also returns `tainted bytes`
when the value can contain outside text. That is the case for a `mixed` field, for example. When your
program wrote the bytes itself, you call `assertTrustedBytes()` once, and then `decode()` reads them.

Every call needs a second argument: a short reason that says what was checked. The program never
reads it. The bytes do not change at all.

It works like `Core\Taint::assertTrusted`, which does the same for a `tainted string`.

**In plain words:** a signed note from you. It says "my program wrote these bytes", and anyone who
reads the code can find it by name.

**The examples below** show an object saved and read back, the bytes staying exactly the same, and
a list of jobs kept in memory and read later.
