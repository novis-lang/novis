The four spellings Unicode allows for the same text, and the one `Core\Str::normalize` rewrites into.

Some characters can be written in more than one way. `é` can be a single character, or the letter `e`
followed by a mark that adds the accent. The two look identical on screen and are not the same text,
so a comparison says they differ and a search for one does not find the other.

**In plain words:** normalizing is agreeing on one spelling before you store text or compare it.

`Nfc` composes, so `é` becomes one character, and it is the form to store and compare in. `Nfd` goes
the other way and keeps the accent separate. Both keep everything, so text survives a round trip
between them.

`Nfkc` and `Nfkd` compose and decompose the same way, and also unify characters that merely look
alike: `ﬁ` becomes `fi`, `①` becomes `1`. That throws a distinction away, which is what you want for
a search key and not what you want for text you are keeping.
