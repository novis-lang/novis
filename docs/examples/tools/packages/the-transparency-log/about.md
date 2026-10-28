A registry keeps a log of every change it makes. Each publish, retraction and change of owner is one
entry. An entry is never changed or removed, so the log only grows.

A checkpoint is the signed summary of the log. It has the name of the log, its number of entries and
a SHA-256 hash of all the entries. The registry signs it with an Ed25519 key. The `key` of the
registry's line under `[registries]` is the public key that checks the signature.

Every `nvs pkg` command that reads a registry checks three things. The new log contains every entry
of the log it saw last time. Each archive it uses is in the log. Each index file has a valid
signature. A file or an archive that fails a check is not used. A log that does not contain the
old entries stops the command with an error.

`nvs pkg add`, `nvs pkg update` and `nvs pkg outdated` also need an index file signed in the last
seven days.

**The examples below** show a program reading a checkpoint, checking its signature with the key from
`[registries]`, and computing its root hash from the entries.
