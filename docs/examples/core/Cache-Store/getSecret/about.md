Reads a secret out of a cache store, and can fetch it when it is not there.

`getSecret` opens the entry that `Core\Cache\Store::putSecret` sealed. Every key of `$keys` is
tried, so an entry an older key sealed still opens after you put a new key at the front of the list.

The result is `null` when nothing opens: no entry under that name, an entry whose lifetime is over,
or an entry none of your keys opens. Each of these is a miss and not an error, so a program that
replaced its keys simply fetches the value again.

The `fill` option is a function that fetches the value. It runs on a miss, and only one caller in
this process runs it while the others wait. It returns a `Core\Cache\SecretEntry`, which carries the
secret and how long that secret stays good. The `wait` option is how long this caller waits.

**The examples below** show a read and a miss, then a `fill` that fetches on a miss, then an access
token that every request needs.
