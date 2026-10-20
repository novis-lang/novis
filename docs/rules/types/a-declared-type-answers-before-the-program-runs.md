`->` on a receiver whose declared type can hold no object is refused where it is written: `$i->name`
where `$i` is declared `int` is `E0495`. A union with a member that is not a class takes the same code,
having no one property set to resolve against. A union of classes is not refused here: a property every
class declares is read through it, and one they do not all share is `E0881`
(`rule:types/a-member-every-class-of-a-union-declares`). The receiver's declaration has ruled the question out, so the
access is dead code that reads as a live one — the same call
`rule:expressions/disjoint-comparison-refused` makes for `==` over two statically disjoint types.

**What is refused is a receiver that can hold no object at all, not an access whose outcome is
knowable.** The type test is the other way round and refuses nothing: `$x is T` is applicable to every
subject, and an answer its declaration settles folds to a constant rather than reporting
(`rule:types/type-test`).

`mixed` is the exception and is answered at run time: it is the one unchecked position, so `$m->name`
defers to `rule:types/erased-member-access`'s name-keyed fetch, which throws for a receiver that turns
out not to be an object and for a name its class does not carry.
