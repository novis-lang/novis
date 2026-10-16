Builds the entry that a `Core\Cache\Store::getSecret` fill hands back.

A fill is a function you give to `getSecret`. It runs when the cache has no secret under the name you
asked for, and it returns one of these entries. The entry carries two things: the secret the fill
fetched, and how long that secret stays good.

The lifetime belongs in the entry because the place you fetched from is the only one that knows it. A
token endpoint says how long its token lasts, so pass that answer on instead of guessing a lifetime
in your own code.

The entry returns nothing itself. Your program builds one for a fill to hand back, and reads the
secret out of `getSecret`.

**The examples below** show a fill that builds one entry, then a lifetime for each of two sources,
then a token endpoint whose own answer becomes the lifetime.
