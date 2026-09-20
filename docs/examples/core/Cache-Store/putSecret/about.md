Stores a secret in a cache store, sealed with one of your keys.

`putSecret` encrypts the value before it reaches the store. The store holds the encrypted bytes and
nothing else, so code that knows the name of the entry but not your keys cannot read the value.

`$keys` is a list of keys, newest first. The first key of the list seals the value, and every key of
the list can open it again. So you can put a new key at the front and still read what an older key
sealed. `Core\Crypto::generateKey` makes a key.

The lifetime is required here. It is sealed into the entry as well as given to the store, so an
entry a store kept for too long is still gone.

Read the value back with `Core\Cache\Store::getSecret`. A plain `Core\Cache\Store::get` on the same
name returns `null`.

**The examples below** show one secret written and read back, then a key list after a rotation, then
an access token kept for as long as the service said it lasts.
